from __future__ import annotations

import argparse
import ctypes
import csv
import json
import os
import time
from pathlib import Path
from typing import Any
from ctypes import wintypes

from playwright.sync_api import Page, sync_playwright


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Exercise a real iTime WebView through CDP.")
    parser.add_argument("--cdp-url", required=True)
    parser.add_argument("--host-pid", required=True, type=int)
    parser.add_argument("--isolated-root", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument("--screenshot", required=True)
    return parser.parse_args()


def invoke(page: Page, command: str, arguments: dict[str, Any] | None = None) -> Any:
    result = page.evaluate(
        """async ({ command, arguments }) => {
          try {
            const value = await window.__TAURI_INTERNALS__.invoke(command, arguments || {});
            return { ok: true, value };
          } catch (error) {
            return { ok: false, error: String(error) };
          }
        }""",
        {"command": command, "arguments": arguments or {}},
    )
    if not result["ok"]:
        raise AssertionError(f"{command} failed: {result['error']}")
    return result["value"]


def rejected_invoke(
    page: Page,
    command: str,
    arguments: dict[str, Any] | None = None,
) -> str:
    result = page.evaluate(
        """async ({ command, arguments }) => {
          try {
            await window.__TAURI_INTERNALS__.invoke(command, arguments || {});
            return { rejected: false, error: "" };
          } catch (error) {
            return { rejected: true, error: String(error) };
          }
        }""",
        {"command": command, "arguments": arguments or {}},
    )
    if not result["rejected"]:
        raise AssertionError(f"{command} unexpectedly accepted invalid input")
    return result["error"]


def ensure_within(path: Path, parent: Path) -> Path:
    resolved = path.resolve()
    if os.path.commonpath([resolved, parent.resolve()]) != str(parent.resolve()):
        raise AssertionError(f"path escaped isolated runtime: {resolved}")
    return resolved


def select_app_page(pages: list[Page]) -> Page:
    candidates = [page for page in pages if "iTime" in page.title()]
    if not candidates:
        details = [{"title": page.title(), "url": page.url} for page in pages]
        raise AssertionError(f"iTime WebView was not found: {details}")
    return candidates[0]


def check_button(page: Page, name: str) -> None:
    locator = page.get_by_role("button", name=name, exact=True)
    if locator.count() != 1 or not locator.is_visible():
        raise AssertionError(f"expected one visible button: {name}")


def has_visible_host_window(process_id: int) -> bool:
    visible = False
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    enum_callback = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)

    def inspect_window(window: int, _: int) -> bool:
        nonlocal visible
        owner_process_id = wintypes.DWORD()
        user32.GetWindowThreadProcessId(window, ctypes.byref(owner_process_id))
        title_length = user32.GetWindowTextLengthW(window)
        title = ctypes.create_unicode_buffer(title_length + 1)
        user32.GetWindowTextW(window, title, title_length + 1)
        if (
            owner_process_id.value == process_id
            and title.value == "iTime"
            and user32.IsWindowVisible(window)
        ):
            visible = True
            return False
        return True

    user32.EnumWindows(enum_callback(inspect_window), 0)
    return visible


def wait_for_host_visibility(process_id: int, expected: bool) -> None:
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if has_visible_host_window(process_id) is expected:
            return
        time.sleep(0.1)
    raise AssertionError(f"host window visibility did not become {expected}")


def main() -> int:
    args = parse_args()
    isolated_root = Path(args.isolated_root).resolve()
    report_path = Path(args.report).resolve()
    screenshot_path = Path(args.screenshot).resolve()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    screenshot_path.parent.mkdir(parents=True, exist_ok=True)

    report: dict[str, Any] = {
        "schemaVersion": 1,
        "passed": False,
        "checks": {},
    }

    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.connect_over_cdp(args.cdp_url)
            pages = [page for context in browser.contexts for page in context.pages]
            page = select_app_page(pages)
            page.wait_for_function(
                "() => Boolean(window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke)",
                timeout=15_000,
            )

            initial_recording = invoke(page, "get_recording_state")
            if not isinstance(initial_recording, bool):
                raise AssertionError("recording state was not boolean")
            if invoke(page, "set_recording_state", {"recording": False}) is not False:
                raise AssertionError("pause command was not confirmed by the backend")
            if invoke(page, "get_recording_state") is not False:
                raise AssertionError("recording stayed enabled after pause")
            if invoke(page, "set_recording_state", {"recording": True}) is not True:
                raise AssertionError("resume command was not confirmed by the backend")
            if invoke(page, "get_recording_state") is not True:
                raise AssertionError("recording stayed disabled after resume")
            report["checks"]["recordingTransitions"] = {
                "initial": initial_recording,
                "pauseConfirmed": True,
                "resumeConfirmed": True,
            }

            consent = invoke(page, "get_provider_consent")
            expected_consent = {
                "version": 1,
                "noticeSeen": False,
                "codexEnabled": False,
                "claudeEnabled": False,
            }
            if consent != expected_consent:
                raise AssertionError(f"isolated Provider consent was not default-off: {consent}")
            provider = invoke(
                page,
                "get_provider_activity_snapshot",
                {"start": 0, "end": 4_102_444_800_000},
            )
            if provider.get("scannedFiles") != 0:
                raise AssertionError("unauthorized Provider snapshot scanned files")
            report["checks"]["providerDefaultOff"] = {
                "consent": consent,
                "scannedFiles": provider.get("scannedFiles"),
                "intervals": len(provider.get("intervals", [])),
            }

            status = invoke(page, "get_local_data_status")
            data_root = ensure_within(Path(status["directory"]), isolated_root)
            retained = invoke(page, "set_data_retention", {"retentionDays": 90})
            if retained.get("retentionDays") != 90:
                raise AssertionError("90-day retention was not persisted")
            permanent = invoke(page, "set_data_retention", {"retentionDays": None})
            if permanent.get("retentionDays") is not None:
                raise AssertionError("permanent retention was not restored")

            exports: dict[str, Any] = {}
            for file_format in ("json", "csv"):
                exported = invoke(page, "export_local_data", {"format": file_format})
                export_path = ensure_within(Path(exported["path"]), data_root)
                if not export_path.is_file() or export_path.stat().st_size != exported["bytes"]:
                    raise AssertionError(f"{file_format} export was not written atomically")
                if file_format == "json":
                    document = json.loads(export_path.read_text(encoding="utf-8"))
                    if document.get("version") != 1:
                        raise AssertionError("JSON export schema version was invalid")
                else:
                    with export_path.open("r", encoding="utf-8", newline="") as stream:
                        header = next(csv.reader(stream))
                    if header[:3] != ["recordType", "start", "end"]:
                        raise AssertionError("CSV export header was invalid")
                exports[file_format] = {
                    "bytes": exported["bytes"],
                    "activityRecords": exported["activityRecords"],
                    "keyboardRecords": exported["keyboardRecords"],
                    "insideIsolatedData": True,
                }

            invalid_confirmation = rejected_invoke(
                page,
                "clear_local_data",
                {"confirmation": "WRONG"},
            )
            if "删除确认无效" not in invalid_confirmation:
                raise AssertionError("invalid deletion confirmation returned the wrong error")
            cleared = invoke(
                page,
                "clear_local_data",
                {"confirmation": "DELETE_ALL_LOCAL_DATA"},
            )
            if cleared.get("activityRecords") != 0 or cleared.get("keyboardRecords") != 0:
                raise AssertionError("confirmed local-data deletion left records behind")
            for file_format in ("json", "csv"):
                exported_files = list((data_root / "Exports").glob(f"*.{file_format}"))
                if not exported_files:
                    raise AssertionError(f"{file_format} exports were deleted with local records")
            report["checks"]["localDataControls"] = {
                "isolatedDataDirectory": True,
                "retention90AndPermanent": True,
                "exports": exports,
                "invalidDeleteRejected": True,
                "confirmedDeleteClearedRecords": True,
                "exportsPreserved": True,
            }

            invoke(
                page,
                "configure_reminders",
                {
                    "enabled": True,
                    "intervalMinutes": 25,
                    "quietStart": "22:00",
                    "quietEnd": "08:00",
                },
            )
            invoke(
                page,
                "configure_reminders",
                {
                    "enabled": False,
                    "intervalMinutes": 25,
                    "quietStart": "22:00",
                    "quietEnd": "08:00",
                },
            )
            report["checks"]["reminderConfiguration"] = {
                "enabled": True,
                "disabled": True,
            }

            page.evaluate("location.hash = '#/settings'")
            page.wait_for_url("**/#/settings", timeout=10_000)
            page.locator(".local-data-section").wait_for(state="visible", timeout=15_000)
            for name in ("打开目录", "导出 JSON", "导出 CSV", "删除全部"):
                check_button(page, name)
            if page.locator(".provider-consent").count() != 1:
                raise AssertionError("Provider permission notice was not visible")
            page.get_by_role(
                "button",
                name="我已了解，选择数据源",
                exact=True,
            ).click()
            page.locator(".provider-source-status").wait_for(
                state="visible",
                timeout=5_000,
            )
            if page.locator(".provider-source-status").count() != 1:
                raise AssertionError("Provider source status was not visible")
            page.screenshot(path=str(screenshot_path), full_page=True)
            report["checks"]["nativeSettingsSurface"] = {
                "localDataActionsVisible": True,
                "providerPermissionNoticeVisible": True,
                "providerStatusVisible": True,
                "screenshot": screenshot_path.name,
            }

            page.evaluate("location.hash = '#/home'")
            page.wait_for_url("**/#/home", timeout=10_000)
            close_button = page.get_by_role("button", name="关闭", exact=True)
            if close_button.count() != 1:
                raise AssertionError("native close button was not unique")
            close_button.click()
            dialog = page.get_by_role("dialog", name="继续在托盘中运行？", exact=True)
            dialog.wait_for(state="visible", timeout=5_000)
            hide_button = dialog.get_by_role("button", name="隐藏到托盘", exact=True)
            if hide_button.count() != 1:
                raise AssertionError("hide-to-tray action was not unique")
            hide_button.click()
            wait_for_host_visibility(args.host_pid, False)
            invoke(page, "plugin:window|show", {"label": "main"})
            invoke(page, "plugin:window|set_focus", {"label": "main"})
            wait_for_host_visibility(args.host_pid, True)
            report["checks"]["windowLifecycle"] = {
                "closePromptVisible": True,
                "hiddenToTray": True,
                "restoredAndFocused": True,
            }

            if initial_recording is False:
                invoke(page, "set_recording_state", {"recording": False})
            report["passed"] = True
            browser.close()
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        report_path.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

    if not report["passed"]:
        raise SystemExit(report.get("error", "native smoke failed"))
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
