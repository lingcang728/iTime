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
sizes = [(960, 680), (1024, 720), (1100, 720), (1180, 760), (1280, 800)]
fit_pages = set()


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
        "const content=document.querySelector('.page-viewport');"
        "return {horizontalOverflow:root.scrollWidth>root.clientWidth+1,verticalOverflow:content.scrollHeight>content.clientHeight+1,clipped,viewport:{width:innerWidth,height:innerHeight},content:{clientHeight:content.clientHeight,scrollHeight:content.scrollHeight}};"
        "}"
    )
    return page.evaluate(script)


def inspect_contrast(page):
    script = (
        "() => {"
        "const parse=(value)=>{const clean=value.trim();if(clean.startsWith('#')){const hex=clean.slice(1);const full=hex.length===3?[...hex].map((part)=>part+part).join(''):hex;return [parseInt(full.slice(0,2),16),parseInt(full.slice(2,4),16),parseInt(full.slice(4,6),16),255]}const match=clean.match(/[\\d.]+/g);if(!match)return null;const values=match.slice(0,4).map(Number);if(clean.startsWith('color(')){return [values[0]*255,values[1]*255,values[2]*255,(values[3]??1)*255]}return values};"
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


report = {"pages": {}, "scroll": {}, "minimumMatrix": [], "selectedMinimum": None, "interactions": {}, "dpi": [], "darkContrast": {}}
channel = os.environ.get("ITIME_BROWSER_CHANNEL", "chrome")

with sync_playwright() as playwright:
    browser = playwright.chromium.launch(channel=channel, headless=True)
    try:
        wide = browser.new_context(viewport={"width": 1180, "height": 760}, locale="zh-CN", timezone_id="Asia/Shanghai", reduced_motion="reduce")
        page = wide.new_page()
        for page_id in pages:
            page.goto(url(page_id))
            wait_ready(page)
            page.screenshot(path=output / f"wide-{page_id}.png")
            report["pages"][page_id] = inspect_layout(page)
            scroll_checks = []
            for ratio in [0.5, 1.0]:
                reached = page.evaluate("ratio => { const content=document.querySelector('.page-viewport'); const target=(content.scrollHeight-content.clientHeight)*ratio; content.scrollTo(0,target); return {top:content.scrollTop,max:content.scrollHeight-content.clientHeight}; }", ratio)
                page.wait_for_timeout(80)
                page.screenshot(path=output / f"wide-{page_id}-scroll-{int(ratio * 100)}.png")
                scroll_checks.append({"ratio": ratio, "reached": reached, "layout": inspect_layout(page)})
            report["scroll"][page_id] = scroll_checks
            page.evaluate("document.querySelector('.page-viewport').scrollTo(0, 0)")

        page.goto(url("ai"))
        wait_ready(page)
        page.locator(".metric__info").first.focus()
        page.wait_for_timeout(200)
        report["interactions"]["agentInfoTooltip"] = page.locator('.metric__info [role="tooltip"]').first.evaluate("element => getComputedStyle(element).opacity === '1'")
        page.locator(".ai-tool-table__row button").first.click()
        page.wait_for_selector(".ai-drawer")
        report["interactions"]["aiDetailDrawer"] = all(page.locator(".ai-drawer").get_by_text(label, exact=True).count() for label in ["Provider 执行", "静默等待", "并行重叠", "检测依据", "检测置信度"])
        report["interactions"]["aiMetricDefinitions"] = page.locator(".ai-drawer").get_by_text("不是工具的“知性度”", exact=False).count() == 1
        page.locator(".ai-drawer__close").click()

        page.goto(url("input"))
        wait_ready(page)
        input_points = page.locator(".spark-point")
        input_points.first.focus()
        report["interactions"]["inputTrendInteraction"] = (
            input_points.count() > 1
            and page.locator(".spark-tooltip").count() == 1
        )
        rhythm_points = page.locator(".rhythm-bars button")
        rhythm_points.first.focus()
        report["interactions"]["inputRhythmInteraction"] = (
            rhythm_points.count() > 1
            and rhythm_points.first.locator('[role="tooltip"]').evaluate("element => getComputedStyle(element).opacity === '1'")
        )

        page.goto(url("timeline"))
        wait_ready(page)
        timeline_segments = page.locator(".activity-lane .lane-segment")
        timeline_segments.first.focus()
        report["interactions"]["timelineInteraction"] = (
            timeline_segments.count() > 0
            and timeline_segments.first.locator('[role="tooltip"]').evaluate("element => getComputedStyle(element).opacity === '1'")
            and page.get_by_text("输入密度", exact=True).count() == 0
        )

        page.goto(url("settings"))
        wait_ready(page)
        starting_url = page.url
        page.locator('.theme-options input[value="light"]').focus()
        page.keyboard.press("Control+PageDown")
        page.wait_for_timeout(100)
        report["interactions"]["shortcutGuard"] = page.url == starting_url
        page.locator("body").click(position={"x": 900, "y": 700})
        page.keyboard.press("Control+PageUp")
        page.wait_for_timeout(100)
        report["interactions"]["globalShortcut"] = "#/goals" in page.url
        page.goto(url("home"))
        page.evaluate("localStorage.removeItem('itime-home-dismissed-reminders-v1')")
        page.reload()
        wait_ready(page)
        report["interactions"]["sidebarProfile"] = page.locator(".profile-card").count() == 1 and page.locator(".recording-state").count() == 0
        ranking_rows = page.locator(".ranking-row")
        report["interactions"]["categoryTimeAndShare"] = (
            ranking_rows.count() > 0
            and ranking_rows.first.locator(".rank-value strong").count() == 1
            and ranking_rows.first.locator(".rank-value b").inner_text().endswith("%")
        )
        semantic_classes = page.locator(".today-timeline .timeline-segment").evaluate_all("elements => [...new Set(elements.flatMap(element => [...element.classList]))]")
        axis_labels = page.locator(".today-timeline .time-axis span").all_inner_texts()
        report["interactions"]["homeTimelineSemantics"] = all(value in semantic_classes for value in ["is-attention", "is-interaction", "is-media"]) and axis_labels == ["00:00", "04:00", "08:00", "12:00", "16:00", "20:00", "24:00"]
        reminder = page.get_by_role("button", name="知道了")
        reminder_visible = reminder.count() == 1
        if reminder_visible:
            reminder.click()
        report["interactions"]["staleReminderSuppressed"] = not reminder_visible

        page.locator(".profile-card").focus()
        page.keyboard.press("Enter")
        page.wait_for_url("**#/settings")
        page.goto(f"{base_url}/#/account")
        page.wait_for_url("**#/home")
        report["interactions"]["localProfileOnly"] = page.get_by_text("本机数据", exact=True).count() == 1 and "#/home" in page.url

        page.goto(url("home"))
        wait_ready(page)
        page.locator(".window-controls .close").click()
        page.wait_for_selector(".close-dialog")
        close_focused = page.locator(".close-dialog").evaluate("dialog => dialog.contains(document.activeElement)")
        page.keyboard.press("Escape")
        page.wait_for_selector(".close-dialog", state="detached")
        report["interactions"]["closePrompt"] = close_focused and page.locator(".close-dialog").count() == 0 and page.locator(".window-controls .close").evaluate("button => document.activeElement === button")

        page.goto(url("goals"))
        wait_ready(page)
        learning_goal = page.locator('.target-form input').first
        learning_goal.fill("135")
        page.get_by_role("button", name="保存目标").click()
        report["interactions"]["editableGoals"] = page.get_by_text("目标已保存", exact=False).count() > 0

        page.goto(url("settings"))
        wait_ready(page)
        report["interactions"]["realSettingsSources"] = (
            page.get_by_text("开机自启动", exact=True).count() == 1
            and page.get_by_text("KeyStats 本机数据", exact=True).count() == 1
            and page.get_by_text("演示迁移", exact=False).count() == 0
        )

        page.goto(url("weekly"))
        wait_ready(page)
        heatmap_cells = page.locator(".heatmap-cell")
        heatmap_cells.first.focus()
        page.wait_for_selector(".heatmap-tooltip")
        first_label = heatmap_cells.first.get_attribute("aria-label")
        page.keyboard.press("ArrowRight")
        focused_label = page.locator(".heatmap-cell:focus").get_attribute("aria-label")
        page.locator(".heatmap-cell:focus").click()
        report["interactions"]["heatmapInteraction"] = heatmap_cells.count() == 49 and first_label != focused_label and page.locator(".heatmap-cell.locked").count() == 1 and page.locator(".heatmap-tooltip").count() == 1
        trend_points = page.locator('.insight-panel .trend g[role="img"]')
        trend_points.first.focus()
        report["interactions"]["weeklyTrendInteraction"] = trend_points.count() == 7 and page.locator(".insight-panel .trend__tooltip").count() == 1
        wide.close()

        compact = browser.new_context(viewport={"width": 560, "height": 900}, locale="zh-CN", timezone_id="Asia/Shanghai", reduced_motion="reduce")
        compact_page = compact.new_page()
        for page_id in ["home", "ai", "weekly"]:
            compact_page.goto(url(page_id, "reference=compact&theme=light"))
            wait_ready(compact_page)
            compact_page.screenshot(path=output / f"reference-{page_id}.png")
        compact.close()

        dark = browser.new_context(viewport={"width": 1280, "height": 800}, color_scheme="dark", locale="zh-CN", reduced_motion="reduce")
        dark_page = dark.new_page()
        for page_id in pages:
            dark_page.goto(url(page_id, "theme=dark"))
            wait_ready(dark_page)
            dark_page.screenshot(path=output / f"dark-{page_id}.png")
            report["darkContrast"][page_id] = inspect_contrast(dark_page)
        dark.close()

        for scale in [1.25, 1.5, 2.0]:
            context = browser.new_context(viewport={"width": 1280, "height": 800}, device_scale_factor=scale, locale="zh-CN", reduced_motion="reduce")
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
                if (width, height) in [(1180, 760), (1280, 800)] and page_id in fit_pages and layout["verticalOverflow"]:
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
wide_passed = all(not value["horizontalOverflow"] and not value["clipped"] and (page_id not in fit_pages or not value["verticalOverflow"]) for page_id, value in report["pages"].items())
dpi_passed = all(not value["horizontalOverflow"] and not value["clipped"] for value in report["dpi"])
scroll_passed = all(
    all(not check["layout"]["horizontalOverflow"] and not check["layout"]["clipped"] and (check["ratio"] < 1 or abs(check["reached"]["top"] - check["reached"]["max"]) <= 1) for check in checks)
    for checks in report["scroll"].values()
)
contrast_passed = all(value["passed"] for value in report["darkContrast"].values())
passed = report["selectedMinimum"] is not None and required_interactions and wide_passed and scroll_passed and dpi_passed and contrast_passed
print(json.dumps({"selectedMinimum": report["selectedMinimum"], "interactions": report["interactions"], "widePassed": wide_passed, "scrollPassed": scroll_passed, "dpiPassed": dpi_passed, "contrastPassed": contrast_passed, "passed": passed}, ensure_ascii=False, indent=2))
sys.exit(0 if passed else 1)
