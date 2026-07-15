import json
import os
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright


root = Path(__file__).resolve().parents[1]
output = root / "artifacts" / "visual"
output.mkdir(parents=True, exist_ok=True)
base_url = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:1420"
pages = ["home", "ai", "timeline", "input", "weekly", "goals", "settings"]
sizes = [(960, 680), (1024, 720), (1100, 720), (1180, 760)]


def url(page, query="theme=light"):
    return f"{base_url}/?{query}#/{page}"


def wait_ready(page):
    page.wait_for_selector(".page")
    page.wait_for_timeout(250)


def inspect_layout(page):
    script = (
        "() => {"
        "const root=document.documentElement;"
        "const interactive=[...document.querySelectorAll('button, a, input, select, textarea, [tabindex=\"0\"]')];"
        "const clipped=interactive.filter((element)=>{"
        "const rect=element.getBoundingClientRect();"
        "const style=getComputedStyle(element);"
        "if(style.display==='none'||style.visibility==='hidden'||rect.width===0||rect.height===0)return false;"
        "return rect.left < -1 || rect.right > innerWidth + 1;"
        "}).map((element)=>element.getAttribute('aria-label')||element.textContent.trim().slice(0,40));"
        "return {horizontalOverflow:root.scrollWidth>root.clientWidth+1,clipped,viewport:{width:innerWidth,height:innerHeight}};"
        "}"
    )
    return page.evaluate(script)


def inspect_contrast(page):
    script = (
        "() => {"
        "const parse=(value)=>{const clean=value.trim();if(clean.startsWith('#')){const hex=clean.slice(1);const full=hex.length===3?[...hex].map((part)=>part+part).join(''):hex;return [parseInt(full.slice(0,2),16),parseInt(full.slice(2,4),16),parseInt(full.slice(4,6),16),255]}const match=clean.match(/[\\d.]+/g);return match?match.slice(0,4).map(Number):null};"
        "const linear=(value)=>{const channel=value/255;return channel<=0.03928?channel/12.92:Math.pow((channel+0.055)/1.055,2.4)};"
        "const lum=(rgb)=>0.2126*linear(rgb[0])+0.7152*linear(rgb[1])+0.0722*linear(rgb[2]);"
        "const ratio=(a,b)=>{const first=lum(a),second=lum(b);return (Math.max(first,second)+0.05)/(Math.min(first,second)+0.05)};"
        "const background=(element)=>{let node=element;while(node){const rgba=parse(getComputedStyle(node).backgroundColor);if(rgba&&rgba.length<4||rgba&&rgba[3]>0.9)return rgba;node=node.parentElement}return [27,29,28]};"
        "const failures=[];"
        "for(const element of document.querySelectorAll('body *')){"
        "const own=[...element.childNodes].some((node)=>node.nodeType===3&&node.textContent.trim());"
        "const rect=element.getBoundingClientRect();if(!own||rect.width===0||rect.height===0||rect.bottom<0||rect.top>innerHeight||rect.right<0||rect.left>innerWidth)continue;"
        "const style=getComputedStyle(element);if(style.display==='none'||style.visibility==='hidden')continue;"
        "const foreground=parse(style.color),back=background(element);if(!foreground||!back)continue;"
        "const size=parseFloat(style.fontSize),weight=parseInt(style.fontWeight)||400;"
        "const threshold=size>=24||(size>=18.66&&weight>=700)?3:4.5;const value=ratio(foreground,back);"
        "if(value+0.01<threshold)failures.push({text:element.textContent.trim().slice(0,45),ratio:Number(value.toFixed(2)),threshold});"
        "}"
        "const root=getComputedStyle(document.documentElement),surface=parse(root.getPropertyValue('--bg-surface'));"
        "const graph=['--accent-blue','--accent-green','--accent-violet','--accent-cyan','--accent-orange'].map((name)=>({name,ratio:Number(ratio(parse(root.getPropertyValue(name)),surface).toFixed(2))}));"
        "return {failures:failures.slice(0,30),graph,passed:failures.length===0&&graph.every((item)=>item.ratio>=3)};"
        "}"
    )
    return page.evaluate(script)


report = {"pages": {}, "minimumMatrix": [], "selectedMinimum": None, "interactions": {}, "dpi": [], "darkContrast": None}
channel = os.environ.get("ITIME_BROWSER_CHANNEL", "chrome")

with sync_playwright() as playwright:
    browser = playwright.chromium.launch(channel=channel, headless=True)
    try:
        wide = browser.new_context(viewport={"width": 1280, "height": 820}, locale="zh-CN", timezone_id="Asia/Shanghai", reduced_motion="reduce")
        page = wide.new_page()
        for page_id in pages:
            page.goto(url(page_id))
            wait_ready(page)
            page.screenshot(path=output / f"wide-{page_id}.png")
            report["pages"][page_id] = inspect_layout(page)

        page.goto(url("ai"))
        wait_ready(page)
        page.locator(".tools-table__row").first.click()
        page.wait_for_selector(".detail-drawer")
        report["interactions"]["aiDetailDrawer"] = all(page.locator(".detail-drawer").get_by_text(label).count() for label in ["有效代理工时", "前台活动", "统计方式", "检测依据", "置信度"])
        page.locator(".drawer-close").click()

        page.goto(url("timeline"))
        wait_ready(page)
        page.locator(".density-toggle").click()
        report["interactions"]["inputDensityToggle"] = page.locator(".density-lane").count() == 1

        page.goto(url("settings"))
        wait_ready(page)
        starting_url = page.url
        page.locator('.theme-options input[value="light"]').focus()
        page.keyboard.press("Control+PageDown")
        page.wait_for_timeout(100)
        report["interactions"]["shortcutGuard"] = page.url == starting_url
        page.locator("body").click(position={"x": 900, "y": 780})
        page.keyboard.press("Control+PageUp")
        page.wait_for_timeout(100)
        report["interactions"]["globalShortcut"] = "#/goals" in page.url
        page.goto(url("home"))
        wait_ready(page)
        page.locator(".recording-state").click()
        report["interactions"]["recordingStatus"] = page.locator(".recording-state").get_by_text("已暂停", exact=True).count() == 1
        page.locator(".recording-state").click()
        page.locator(".window-controls .close").click()
        page.wait_for_selector(".close-dialog")
        report["interactions"]["closePrompt"] = page.locator(".close-dialog").get_by_text("继续在托盘中运行？", exact=True).count() == 1
        page.locator(".close-dialog .secondary").click()
        page.goto(url("settings"))
        wait_ready(page)
        page.get_by_role("button", name="演示迁移").click()
        report["interactions"]["migrationDemo"] = page.get_by_text("迁移演示完成", exact=True).count() == 1
        wide.close()

        compact = browser.new_context(viewport={"width": 560, "height": 900}, locale="zh-CN", timezone_id="Asia/Shanghai", reduced_motion="reduce")
        compact_page = compact.new_page()
        for page_id in ["home", "ai", "weekly"]:
            compact_page.goto(url(page_id, "reference=compact&theme=light"))
            wait_ready(compact_page)
            compact_page.screenshot(path=output / f"reference-{page_id}.png")
        compact.close()

        dark = browser.new_context(viewport={"width": 1280, "height": 820}, color_scheme="dark", locale="zh-CN", reduced_motion="reduce")
        dark_page = dark.new_page()
        dark_page.goto(url("weekly", "theme=dark"))
        wait_ready(dark_page)
        dark_page.screenshot(path=output / "dark-weekly.png")
        report["darkContrast"] = inspect_contrast(dark_page)
        dark.close()

        for scale in [1.25, 1.5, 2.0]:
            context = browser.new_context(viewport={"width": 1280, "height": 820}, device_scale_factor=scale, locale="zh-CN", reduced_motion="reduce")
            dpi_page = context.new_page()
            dpi_page.goto(url("home"))
            wait_ready(dpi_page)
            dpi_page.screenshot(path=output / f"dpi-{int(scale * 100)}-home.png")
            layout = inspect_layout(dpi_page)
            layout["scale"] = scale
            report["dpi"].append(layout)
            context.close()

        for width, height in sizes:
            context = browser.new_context(viewport={"width": width, "height": height}, locale="zh-CN", reduced_motion="reduce")
            matrix_page = context.new_page()
            result = {"width": width, "height": height, "pages": {}, "passed": True}
            for page_id in pages:
                matrix_page.goto(url(page_id))
                wait_ready(matrix_page)
                layout = inspect_layout(matrix_page)
                result["pages"][page_id] = layout
                if layout["horizontalOverflow"] or layout["clipped"]:
                    result["passed"] = False
            context.close()
            report["minimumMatrix"].append(result)
            if report["selectedMinimum"] is None and result["passed"]:
                report["selectedMinimum"] = {"width": width, "height": height}
    finally:
        browser.close()

with (output / "layout-report.json").open("w", encoding="utf-8") as handle:
    json.dump(report, handle, ensure_ascii=False, indent=2)

required_interactions = all(report["interactions"].values())
wide_passed = all(not value["horizontalOverflow"] and not value["clipped"] for value in report["pages"].values())
dpi_passed = all(not value["horizontalOverflow"] and not value["clipped"] for value in report["dpi"])
contrast_passed = report["darkContrast"]["passed"]
passed = report["selectedMinimum"] is not None and required_interactions and wide_passed and dpi_passed and contrast_passed
print(json.dumps({"selectedMinimum": report["selectedMinimum"], "interactions": report["interactions"], "widePassed": wide_passed, "dpiPassed": dpi_passed, "contrastPassed": contrast_passed, "passed": passed}, ensure_ascii=False, indent=2))
sys.exit(0 if passed else 1)
