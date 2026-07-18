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
sizes = [(960, 680), (1024, 720), (1100, 720), (1180, 760), (1280, 800), (1540, 944)]
fit_pages = {"home", "timeline"}
base_geometry_selectors = [
    ".desktop-app",
    ".sidebar",
    ".app-surface",
    ".page-header",
]
page_geometry_selectors = {
    "home": [".metrics-grid--home, .metrics-strip", ".home-data-grid"],
    "ai": [".ai-metrics", ".ai-workspace-grid"],
    "timeline": [".timeline-overview", ".full-timeline"],
    "input": [".input-stat-strip", ".input-history-wrap"],
    "weekly": [".weekly-daily-card", ".weekly-analysis-grid", ".weekly-secondary-grid"],
    "goals": [".goal-overview", ".goals-layout"],
    "settings": [".settings-layout"],
}
geometry_threshold_css_px = 1
focus_indicator_threshold = 3
focus_targets = [
    {
        "id": "weeklyTrend",
        "page": "weekly",
        "selector": ".attention-panel .trend g[role=\"img\"][tabindex=\"0\"]",
        "indicator": "circle",
        "indicatorKind": "stroke",
    },
    {"id": "focusHeatmap", "page": "weekly", "selector": ".heat-cell"},
    {"id": "timelineSegment", "page": "timeline", "selector": ".activity-lane .lane-segment"},
    {"id": "goalsInput", "page": "goals", "selector": ".target-form input"},
]


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


def geometry_selectors_for(page_id):
    return [*base_geometry_selectors, *page_geometry_selectors[page_id]]


def inspect_geometry(page, page_id):
    selectors = geometry_selectors_for(page_id)
    script = (
        "(selectors) => Object.fromEntries(selectors.map((selector) => {"
        "const rects=[...document.querySelectorAll(selector)]"
        ".filter((element)=>{const style=getComputedStyle(element);const rect=element.getBoundingClientRect();"
        "return style.display!=='none'&&style.visibility!=='hidden'&&rect.width>0&&rect.height>0})"
        ".map((element)=>{const rect=element.getBoundingClientRect();"
        "return Object.fromEntries(['x','y','width','height'].map((key)=>[key,Number(rect[key].toFixed(3))]))});"
        "return [selector,{count:rects.length,rects}]"
        "}))"
    )
    return page.evaluate(script, selectors)


def compare_theme_geometry(page_id, light, dark):
    comparison = {"passed": True, "selectors": {}}
    for selector in geometry_selectors_for(page_id):
        light_value = light.get(selector, {"count": 0, "rects": []})
        dark_value = dark.get(selector, {"count": 0, "rects": []})
        selector_result = {
            "lightCount": light_value["count"],
            "darkCount": dark_value["count"],
            "rects": [],
            "passed": True,
        }
        if light_value["count"] != dark_value["count"]:
            selector_result["passed"] = False
            selector_result["reason"] = "count-mismatch"
        elif light_value["count"] == 0:
            selector_result["present"] = False
            selector_result["passed"] = False
            selector_result["reason"] = "required-selector-missing"
        else:
            selector_result["present"] = True
            for light_rect, dark_rect in zip(light_value["rects"], dark_value["rects"]):
                deltas = {
                    key: round(abs(light_rect[key] - dark_rect[key]), 3)
                    for key in ["x", "y", "width", "height"]
                }
                max_delta = max(deltas.values())
                selector_result["rects"].append({"deltas": deltas, "maxDelta": max_delta})
                if max_delta > geometry_threshold_css_px:
                    selector_result["passed"] = False
        comparison["selectors"][selector] = selector_result
        if not selector_result["passed"]:
            comparison["passed"] = False
    return comparison


def inspect_contrast(page):
    script = (
        "() => {"
        "const parse=(value)=>{const clean=value.trim();if(!clean||clean==='transparent')return null;"
        "if(clean.startsWith('#')){const hex=clean.slice(1);const full=hex.length===3?[...hex].map((part)=>part+part).join(''):hex;"
        "return [parseInt(full.slice(0,2),16),parseInt(full.slice(2,4),16),parseInt(full.slice(4,6),16),1]}"
        "const match=clean.match(/[\\d.]+/g);if(!match)return null;const values=match.slice(0,4).map(Number);"
        "if(clean.startsWith('color('))return [values[0]*255,values[1]*255,values[2]*255,values[3]??1];"
        "return [values[0],values[1],values[2],values[3]??1]};"
        "const shadowColor=(value)=>{const colors=[...value.matchAll(/(?:rgba?|color)\\([^)]*\\)|#[0-9a-f]{3,8}/gi)];"
        "return colors.length?parse(colors.at(-1)[0]):null};"
        "const linear=(value)=>{const channel=value/255;return channel<=0.03928?channel/12.92:Math.pow((channel+0.055)/1.055,2.4)};"
        "const lum=(rgb)=>0.2126*linear(rgb[0])+0.7152*linear(rgb[1])+0.0722*linear(rgb[2]);"
        "const ratio=(a,b)=>{const first=lum(a),second=lum(b);return (Math.max(first,second)+0.05)/(Math.min(first,second)+0.05)};"
        "const painted=(foreground,back)=>{const alpha=foreground[3]??1;return [foreground[0]*alpha+back[0]*(1-alpha),foreground[1]*alpha+back[1]*(1-alpha),foreground[2]*alpha+back[2]*(1-alpha),1]};"
        "const contrast=(foreground,back)=>ratio(painted(foreground,back),back);"
        "const background=(element)=>{let node=element;while(node){const rgba=parse(getComputedStyle(node).backgroundColor);"
        "if(rgba&&rgba[3]>0.9)return rgba;node=node.parentElement}"
        "return matchMedia('(prefers-color-scheme: dark)').matches?[21,24,23,1]:[247,247,244,1]};"
        "const visible=(element)=>{const rect=element.getBoundingClientRect(),style=getComputedStyle(element);"
        "return rect.width>0&&rect.height>0&&rect.bottom>=0&&rect.top<=innerHeight&&rect.right>=0&&rect.left<=innerWidth&&style.display!=='none'&&style.visibility!=='hidden'&&(parseFloat(style.opacity)||0)>0.01};"
        "const textFailures=[];"
        "for(const element of document.querySelectorAll('body *')){"
        "const own=[...element.childNodes].some((node)=>node.nodeType===3&&node.textContent.trim());"
        "if(!own||!visible(element))continue;const style=getComputedStyle(element);"
        "const foreground=parse(style.color),back=background(element);if(!foreground||!back)continue;"
        "const size=parseFloat(style.fontSize),weight=parseInt(style.fontWeight)||400;"
        "const threshold=size>=24||(size>=18.66&&weight>=700)?3:4.5;const value=contrast(foreground,back);"
        "if(value+0.01<threshold)textFailures.push({text:element.textContent.trim().slice(0,45),ratio:Number(value.toFixed(2)),threshold});"
        "}"
        "const controlFailures=[],controlSelector='button,input:not([type=\"hidden\"]),select,textarea,[role=\"button\"],[role=\"switch\"],[role=\"radio\"],[role=\"checkbox\"]';"
        "for(const element of document.querySelectorAll(controlSelector)){"
        "if(!visible(element)||element.disabled||element.getAttribute('aria-disabled')==='true')continue;"
        "const style=getComputedStyle(element),foreground=parse(style.color),ownBackground=parse(style.backgroundColor);"
        "const parentBackground=background(element.parentElement);"
        "const effectiveBackground=ownBackground?painted(ownBackground,parentBackground):parentBackground;"
        "const border=parse(style.borderColor);"
        "const borderWidth=Math.max(...['Top','Right','Bottom','Left'].map((side)=>parseFloat(style[`border${side}Width`])||0));"
        "const ratios={foreground:foreground?contrast(foreground,effectiveBackground):0,border:border&&borderWidth>0?contrast(border,parentBackground):0,background:ownBackground?ratio(effectiveBackground,parentBackground):0};"
        "if(Math.max(...Object.values(ratios))+0.01<3)controlFailures.push({label:element.getAttribute('aria-label')||element.textContent.trim().slice(0,45)||element.tagName.toLowerCase(),ratios:Object.fromEntries(Object.entries(ratios).map(([name,value])=>[name,Number(value.toFixed(2))])),threshold:3});"
        "}"
        "const visualControls=[],visualControlFailures=[];"
        "for(const track of document.querySelectorAll('.toggle i, .settings-page input[type=\"checkbox\"] + i')){"
        "if(!visible(track))continue;const input=track.previousElementSibling;"
        "if(!input||!input.matches('input[type=\"checkbox\"]')){visualControlFailures.push({label:'toggle',reason:'visible-track-missing-checkbox'});continue}"
        "if(input.disabled||input.getAttribute('aria-disabled')==='true')continue;"
        "const trackStyle=getComputedStyle(track),knobStyle=getComputedStyle(track,'::after');"
        "const parentBackground=background(track.parentElement),trackFill=parse(trackStyle.backgroundColor);"
        "const effectiveTrack=trackFill?painted(trackFill,parentBackground):parentBackground;"
        "const trackBoundary=shadowColor(trackStyle.boxShadow);"
        "const knobFill=parse(knobStyle.backgroundColor),effectiveKnob=knobFill?painted(knobFill,effectiveTrack):effectiveTrack;"
        "const knobBoundary=shadowColor(knobStyle.boxShadow);"
        "const trackRatios={fill:trackFill?ratio(effectiveTrack,parentBackground):0,boundary:trackBoundary?contrast(trackBoundary,parentBackground):0};"
        "const knobRatios={fill:knobFill?ratio(effectiveKnob,effectiveTrack):0,boundary:knobBoundary?contrast(knobBoundary,effectiveTrack):0};"
        "const labelElement=input.closest('label'),label=input.getAttribute('aria-label')||labelElement?.textContent.trim().slice(0,45)||'toggle';"
        "const sample={label,checked:input.checked,track:Object.fromEntries(Object.entries(trackRatios).map(([name,value])=>[name,Number(value.toFixed(2))])),knob:Object.fromEntries(Object.entries(knobRatios).map(([name,value])=>[name,Number(value.toFixed(2))])),threshold:3};"
        "visualControls.push(sample);"
        "if(Math.max(...Object.values(trackRatios))+0.01<3||Math.max(...Object.values(knobRatios))+0.01<3)visualControlFailures.push(sample);"
        "}"
        "const root=getComputedStyle(document.documentElement),surface=parse(root.getPropertyValue('--bg-surface'))||background(document.body);"
        "const graph=['--accent-strong','--neutral-series','--warning','--danger']"
        ".map((name)=>({name,color:parse(root.getPropertyValue(name))})).filter((item)=>item.color)"
        ".map((item)=>({name:item.name,ratio:Number(contrast(item.color,surface).toFixed(2))}));"
        "return {failures:textFailures.slice(0,30),textFailures:textFailures.slice(0,30),controlFailures:controlFailures.slice(0,30),visualControls:visualControls.slice(0,30),visualControlFailures:visualControlFailures.slice(0,30),graph,passed:textFailures.length===0&&controlFailures.length===0&&visualControlFailures.length===0&&graph.every((item)=>item.ratio>=3)};"
        "}"
    )
    return page.evaluate(script)


def inspect_contrast_positions(page):
    original = page.evaluate(
        "() => { const content=document.querySelector('.page-viewport'); "
        "return {top:content.scrollTop,max:Math.max(0,content.scrollHeight-content.clientHeight)} }"
    )
    positions = [("top", 0.0)]
    if original["max"] > 1:
        positions.extend([("middle", 0.5), ("bottom", 1.0)])
    scans = []
    for label, ratio in positions:
        reached = page.evaluate(
            "ratio => { const content=document.querySelector('.page-viewport'); "
            "content.scrollTo(0,(content.scrollHeight-content.clientHeight)*ratio); return content.scrollTop }",
            ratio,
        )
        page.wait_for_timeout(60)
        scan = inspect_contrast(page)
        scan["position"] = label
        scan["scrollTop"] = round(reached, 3)
        scans.append(scan)
    page.evaluate(
        "top => document.querySelector('.page-viewport').scrollTo(0,top)", original["top"]
    )

    def merged(key):
        return [
            {**item, "position": scan["position"], "scrollTop": scan["scrollTop"]}
            for scan in scans
            for item in scan.get(key, [])
        ]

    text_failures = merged("textFailures")
    control_failures = merged("controlFailures")
    visual_controls = merged("visualControls")
    visual_control_failures = merged("visualControlFailures")
    return {
        "scrollable": original["max"] > 1,
        "maxScroll": round(original["max"], 3),
        "expectedPositions": [label for label, _ in positions],
        "positions": scans,
        "failures": text_failures,
        "textFailures": text_failures,
        "controlFailures": control_failures,
        "visualControls": visual_controls,
        "visualControlFailures": visual_control_failures,
        "graph": scans[0]["graph"],
        "passed": all(scan["passed"] for scan in scans),
    }


def inspect_tab_focus(page, target):
    selector = target["selector"]
    count = page.locator(selector).count()
    if count == 0:
        return {"passed": False, "reason": "target-missing", "selector": selector, "count": 0}
    page.evaluate(
        "() => { document.activeElement?.blur(); document.querySelector('.page-viewport').scrollTo(0,0) }"
    )
    tab_presses = None
    for attempt in range(1, 241):
        page.keyboard.press("Tab")
        if page.evaluate("selector => document.activeElement?.matches(selector) === true", selector):
            tab_presses = attempt
            break
    if tab_presses is None:
        return {
            "passed": False,
            "reason": "tab-target-unreachable",
            "selector": selector,
            "count": count,
            "tabPresses": 240,
        }
    page.wait_for_timeout(80)
    result = page.evaluate(
        "({selector,indicatorSelector,indicatorKind,threshold}) => {"
        "const parse=(value)=>{const clean=(value||'').trim();if(!clean||clean==='none'||clean==='transparent')return null;"
        "if(clean.startsWith('#')){const hex=clean.slice(1),full=hex.length===3?[...hex].map((part)=>part+part).join(''):hex;return [parseInt(full.slice(0,2),16),parseInt(full.slice(2,4),16),parseInt(full.slice(4,6),16),1]}"
        "const match=clean.match(/[\\d.]+/g);if(!match)return null;const values=match.slice(0,4).map(Number);"
        "if(clean.startsWith('color('))return [values[0]*255,values[1]*255,values[2]*255,values[3]??1];return [values[0],values[1],values[2],values[3]??1]};"
        "const linear=(value)=>{const channel=value/255;return channel<=0.03928?channel/12.92:Math.pow((channel+0.055)/1.055,2.4)};"
        "const luminance=(rgb)=>0.2126*linear(rgb[0])+0.7152*linear(rgb[1])+0.0722*linear(rgb[2]);"
        "const ratio=(a,b)=>{const first=luminance(a),second=luminance(b);return (Math.max(first,second)+0.05)/(Math.min(first,second)+0.05)};"
        "const painted=(foreground,back)=>{const alpha=foreground[3]??1;return [foreground[0]*alpha+back[0]*(1-alpha),foreground[1]*alpha+back[1]*(1-alpha),foreground[2]*alpha+back[2]*(1-alpha),1]};"
        "const contrast=(foreground,back)=>ratio(painted(foreground,back),back);"
        "const background=(element)=>{let node=element;while(node){const rgba=parse(getComputedStyle(node).backgroundColor);if(rgba&&rgba[3]>0.9)return rgba;node=node.parentElement}"
        "const root=parse(getComputedStyle(document.documentElement).getPropertyValue('--bg-app'));return root||[247,247,244,1]};"
        "const active=document.activeElement;if(!active||!active.matches(selector))return {passed:false,reason:'active-target-mismatch'};"
        "const activeStyle=getComputedStyle(active),indicator=indicatorSelector?active.querySelector(indicatorSelector):active;"
        "if(!indicator)return {passed:false,reason:'indicator-missing'};const indicatorStyle=getComputedStyle(indicator);"
        "const rect=active.getBoundingClientRect(),activeVisible=rect.width>0&&rect.height>0&&rect.bottom>=0&&rect.top<=innerHeight&&rect.right>=0&&rect.left<=innerWidth;"
        "const adjacent=background(active.parentElement),candidates=[];"
        "const add=(kind,color,width,back=adjacent)=>{if(color&&width>0)candidates.push({kind,width:Number(width.toFixed(2)),ratio:Number(contrast(color,back).toFixed(2))})};"
        "const outlineWidth=parseFloat(activeStyle.outlineWidth)||0;"
        "if(activeStyle.outlineStyle!=='none')add('outline',parse(activeStyle.outlineColor),outlineWidth);"
        "const borderWidth=Math.max(...['Top','Right','Bottom','Left'].map((side)=>parseFloat(activeStyle[`border${side}Width`])||0));"
        "if(borderWidth>0)add('border',parse(activeStyle.borderColor),borderWidth);"
        "const strokeWidth=parseFloat(indicatorStyle.strokeWidth)||0;add('stroke',parse(indicatorStyle.stroke),strokeWidth,background(indicator.parentElement));"
        "const requiredCandidates=candidates.filter((candidate)=>candidate.kind===indicatorKind);"
        "const best=Math.max(0,...requiredCandidates.map((candidate)=>candidate.ratio));"
        "return {selector,count:document.querySelectorAll(selector).length,label:active.getAttribute('aria-label')||active.textContent.trim().slice(0,60),"
        "focusVisible:active.matches(':focus-visible'),activeVisible,indicatorKind,candidates,requiredCandidates,bestRatio:Number(best.toFixed(2)),threshold,"
        "passed:active.matches(':focus-visible')&&activeVisible&&requiredCandidates.length>0&&best+0.01>=threshold};"
        "}",
        {
            "selector": selector,
            "indicatorSelector": target.get("indicator"),
            "indicatorKind": target.get("indicatorKind", "outline"),
            "threshold": focus_indicator_threshold,
        },
    )
    result["tabPresses"] = tab_presses
    result["page"] = target["page"]
    return result


def inspect_focus_indicators(page, theme):
    results = {}
    for target in focus_targets:
        page.goto(url(target["page"], f"theme={theme}"))
        wait_ready(page)
        results[target["id"]] = inspect_tab_focus(page, target)
    return results


report = {
    "pages": {},
    "scroll": {},
    "wheelBoundary": {},
    "minimumMatrix": [],
    "selectedMinimum": None,
    "interactions": {},
    "dpi": [],
    "themeContrast": {"light": {}, "dark": {}},
    "focusIndicators": {
        "threshold": focus_indicator_threshold,
        "light": {},
        "dark": {},
    },
    "themeGeometry": {
        "thresholdCssPx": geometry_threshold_css_px,
        "requirements": {page_id: geometry_selectors_for(page_id) for page_id in pages},
        "light": {},
        "dark": {},
        "comparison": {},
    },
}
channel = os.environ.get("ITIME_BROWSER_CHANNEL", "chrome")

with sync_playwright() as playwright:
    browser = playwright.chromium.launch(channel=channel, headless=True)
    try:
        wide = browser.new_context(viewport={"width": 1540, "height": 944}, locale="zh-CN", timezone_id="Asia/Shanghai", reduced_motion="reduce")
        page = wide.new_page()
        for page_id in pages:
            page.goto(url(page_id))
            wait_ready(page)
            page.screenshot(path=output / f"wide-{page_id}.png")
            report["pages"][page_id] = inspect_layout(page)
            report["themeContrast"]["light"][page_id] = inspect_contrast_positions(page)
            report["themeGeometry"]["light"][page_id] = inspect_geometry(page, page_id)
            scroll_checks = []
            for ratio in [0.5, 1.0]:
                reached = page.evaluate("ratio => { const content=document.querySelector('.page-viewport'); const target=(content.scrollHeight-content.clientHeight)*ratio; content.scrollTo(0,target); return {top:content.scrollTop,max:content.scrollHeight-content.clientHeight}; }", ratio)
                page.wait_for_timeout(80)
                page.screenshot(path=output / f"wide-{page_id}-scroll-{int(ratio * 100)}.png")
                scroll_checks.append({"ratio": ratio, "reached": reached, "layout": inspect_layout(page)})
            report["scroll"][page_id] = scroll_checks
            page.evaluate("document.querySelector('.page-viewport').scrollTo(0, 0)")

        for page_id in pages:
            page.goto(url(page_id))
            wait_ready(page)
            viewport = page.locator(".page-viewport")
            bounds = viewport.bounding_box()
            if bounds is None:
                raise RuntimeError(f"{page_id} 页面缺少可见滚动容器")
            page.mouse.move(bounds["x"] + bounds["width"] / 2, bounds["y"] + bounds["height"] / 2)
            for _ in range(8):
                page.mouse.wheel(0, 2400)
            page.wait_for_timeout(120)
            for _ in range(4):
                page.mouse.wheel(0, 2400)
            page.wait_for_timeout(160)
            boundary = page.evaluate(
                "() => {"
                "const content=document.querySelector('.page-viewport');"
                "const rect=(selector)=>{const value=document.querySelector(selector).getBoundingClientRect();"
                "return {top:value.top,bottom:value.bottom,height:value.height}};"
                "return {top:content.scrollTop,max:content.scrollHeight-content.clientHeight,"
                "windowTop:window.scrollY,htmlTop:document.documentElement.scrollTop,bodyTop:document.body.scrollTop,"
                "viewportHeight:innerHeight,app:rect('.desktop-app'),sidebar:rect('.sidebar'),"
                "surface:rect('.app-surface'),content:rect('.page-viewport')};"
                "}"
            )
            boundary["passed"] = (
                abs(boundary["top"] - boundary["max"]) <= 1
                and boundary["windowTop"] == 0
                and boundary["htmlTop"] == 0
                and boundary["bodyTop"] == 0
                and all(abs(boundary[name]["top"]) <= 1 and abs(boundary[name]["bottom"] - boundary["viewportHeight"]) <= 1 for name in ["app", "sidebar", "surface"])
                and abs(boundary["content"]["bottom"] - boundary["viewportHeight"]) <= 1
            )
            report["wheelBoundary"][page_id] = boundary
            page.screenshot(path=output / f"wheel-boundary-{page_id}.png")

        page.goto(url("ai"))
        wait_ready(page)
        metric_info = page.locator(".metric-info, .metric__info").first
        metric_info.focus()
        page.wait_for_timeout(200)
        report["interactions"]["agentInfoTooltip"] = metric_info.locator('[role="tooltip"]').evaluate("element => getComputedStyle(element).opacity === '1'")
        page.locator(".ai-tool-item button").first.click()
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
        report["interactions"]["inputKeyboardOnly"] = (
            page.get_by_text("总输入字数", exact=True).count() == 1
            and page.get_by_text("平均输入字数", exact=True).count() == 1
            and page.get_by_text("输入节奏", exact=True).count() == 0
            and page.get_by_text("鼠标移动", exact=True).count() == 0
            and page.get_by_text("键盘热力图", exact=True).count() == 0
        )

        page.goto(url("timeline"))
        wait_ready(page)
        timeline_segments = page.locator(".activity-lane .lane-segment")
        timeline_segments.first.focus()
        page.wait_for_timeout(140)
        report["interactions"]["timelineInteraction"] = (
            timeline_segments.count() > 0
            and timeline_segments.first.locator('[role="tooltip"]').evaluate("element => getComputedStyle(element).opacity === '1'")
            and page.get_by_text("输入密度", exact=True).count() == 0
            and page.get_by_text("语音输入", exact=True).count() == 0
        )
        page.get_by_role("button", name="查看统计口径与轨道说明").click()
        timeline_notes = page.get_by_role("dialog", name="统计口径与轨道说明")
        report["interactions"]["timelineNotesPopover"] = (
            timeline_notes.get_by_text("总覆盖按自然时间并集计算", exact=True).count() == 1
            and timeline_notes.get_by_text("上下对齐表示同时发生", exact=True).count() == 1
        )
        timeline_notes.get_by_role("button", name="关闭说明").click()

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
            and ranking_rows.first.locator(".rank-duration").inner_text().strip() != ""
            and ranking_rows.first.locator(".rank-value b").inner_text().endswith("%")
        )
        activity_rows = page.locator(".home-activity-row")
        activity_times = activity_rows.locator("time").all_inner_texts()
        report["interactions"]["homeTimelineSemantics"] = (
            activity_rows.count() > 0
            and len(activity_times) == activity_rows.count()
            and activity_times == sorted(activity_times)
            and activity_rows.locator(".home-activity-card .app-icon").count() == activity_rows.count()
        )
        home_metrics = page.locator(".metrics-grid--home, .metrics-strip").first
        home_metric_columns = home_metrics.evaluate("element => getComputedStyle(element).gridTemplateColumns.split(' ').length")
        home_data_columns = page.locator(".home-data-grid").evaluate("element => getComputedStyle(element).gridTemplateColumns.split(' ').length")
        report["interactions"]["homeSelectedFormat"] = (
            home_metrics.locator(".metric-card").count() == 4
            and home_metrics.locator(".metric-card__art").count() == 4
            and home_metric_columns == 4
            and home_data_columns == 2
            and activity_rows.count() >= 4
        )
        reference_system = page.evaluate("""() => {
          const sidebar = document.querySelector('.sidebar').getBoundingClientRect()
          const pageBox = document.querySelector('.page')
          const card = document.querySelector('.metric-card')
          const metrics = document.querySelector('.metrics-grid--home')
          const pageStyle = getComputedStyle(pageBox)
          const cardStyle = getComputedStyle(card)
          const metricsStyle = getComputedStyle(metrics)
          return {
            sidebarWidth: sidebar.width,
            pagePadding: parseFloat(pageStyle.paddingLeft),
            cardRadius: parseFloat(cardStyle.borderRadius),
            metricGap: parseFloat(metricsStyle.columnGap),
            cardBackground: cardStyle.backgroundColor,
          }
        }""")
        report["interactions"]["referenceDesignSystem"] = (
            abs(reference_system["sidebarWidth"] - 240) <= 1
            and abs(reference_system["pagePadding"] - 32) <= 1
            and reference_system["cardRadius"] >= 10
            and abs(reference_system["metricGap"] - 16) <= 1
            and reference_system["cardBackground"] not in ["rgb(255, 255, 255)", "rgb(0, 0, 0)"]
        )
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
        report["interactions"]["localProfileOnly"] = page.get_by_text("数据已同步", exact=True).count() == 1 and "#/home" in page.url

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
            and page.get_by_text("本机键盘计数", exact=True).count() == 1
            and page.get_by_text("Windows 字符键按下计数", exact=True).count() == 1
            and page.get_by_text("演示迁移", exact=False).count() == 0
        )

        page.goto(url("weekly"))
        wait_ready(page)
        weekly_analysis_columns = page.locator(".weekly-analysis-grid").evaluate("element => getComputedStyle(element).gridTemplateColumns.split(' ').length")
        weekly_secondary_columns = page.locator(".weekly-secondary-grid").evaluate("element => getComputedStyle(element).gridTemplateColumns.split(' ').length")
        weekly_analysis_order = page.locator(".weekly-analysis-grid > section").evaluate_all("elements => elements.map(element => [...element.classList].find(value => value.endsWith('-panel')))")
        weekly_secondary_order = page.locator(".weekly-secondary-grid > section").evaluate_all("elements => elements.map(element => [...element.classList].find(value => value.endsWith('-panel')))")
        report["interactions"]["weeklySelectedFormat"] = (
            weekly_analysis_columns == 3
            and weekly_secondary_columns == 2
            and weekly_analysis_order == ["focus-panel", "insight-panel", "top-apps-panel"]
            and weekly_secondary_order == ["attention-panel", "achievements-panel"]
            and page.locator(".weekly-daily-card").count() == 1
            and page.locator(".top-apps-panel").count() == 1
            and page.locator(".weekly-analysis-grid .top-apps-panel").count() == 1
            and page.locator(".weekly-secondary-grid .top-apps-panel").count() == 0
        )
        heatmap_cells = page.locator(".heat-cell")
        heatmap_cells.first.focus()
        page.wait_for_selector(".heat-cell:focus [role='tooltip']")
        first_label = heatmap_cells.first.get_attribute("aria-label")
        page.keyboard.press("ArrowRight")
        focused_label = page.locator(".heat-cell:focus").get_attribute("aria-label")
        page.locator(".heat-cell:focus").click()
        report["interactions"]["heatmapInteraction"] = heatmap_cells.count() == 49 and first_label != focused_label and page.locator(".heat-cell.locked").count() == 1 and page.locator(".heat-cell.locked [role='tooltip']").count() == 1
        trend_points = page.locator('.attention-panel .trend g[role="img"]')
        trend_points.first.focus()
        report["interactions"]["weeklyTrendInteraction"] = trend_points.count() == 7 and page.locator(".attention-panel .trend__tooltip").count() == 1
        report["focusIndicators"]["light"] = inspect_focus_indicators(page, "light")
        wide.close()

        compact = browser.new_context(viewport={"width": 560, "height": 900}, locale="zh-CN", timezone_id="Asia/Shanghai", reduced_motion="reduce")
        compact_page = compact.new_page()
        for page_id in ["home", "ai", "weekly"]:
            compact_page.goto(url(page_id, "reference=compact&theme=light"))
            wait_ready(compact_page)
            compact_page.screenshot(path=output / f"reference-{page_id}.png")
        compact.close()

        dark = browser.new_context(viewport={"width": 1540, "height": 944}, color_scheme="dark", locale="zh-CN", timezone_id="Asia/Shanghai", reduced_motion="reduce")
        dark_page = dark.new_page()
        for page_id in pages:
            dark_page.goto(url(page_id, "theme=dark"))
            wait_ready(dark_page)
            dark_page.screenshot(path=output / f"dark-{page_id}.png")
            report["themeContrast"]["dark"][page_id] = inspect_contrast_positions(dark_page)
            report["themeGeometry"]["dark"][page_id] = inspect_geometry(dark_page, page_id)
            report["themeGeometry"]["comparison"][page_id] = compare_theme_geometry(
                page_id,
                report["themeGeometry"]["light"][page_id],
                report["themeGeometry"]["dark"][page_id],
            )
        report["focusIndicators"]["dark"] = inspect_focus_indicators(dark_page, "dark")
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
contrast_passed = all(
    value["passed"]
    for theme in report["themeContrast"].values()
    for value in theme.values()
)
focus_indicators_passed = all(
    value["passed"]
    for theme in ["light", "dark"]
    for value in report["focusIndicators"][theme].values()
)
theme_geometry_passed = all(value["passed"] for value in report["themeGeometry"]["comparison"].values())
wheel_boundary_passed = all(value["passed"] for value in report["wheelBoundary"].values())
passed = report["selectedMinimum"] is not None and required_interactions and wide_passed and scroll_passed and wheel_boundary_passed and dpi_passed and contrast_passed and focus_indicators_passed and theme_geometry_passed
print(json.dumps({"selectedMinimum": report["selectedMinimum"], "interactions": report["interactions"], "widePassed": wide_passed, "scrollPassed": scroll_passed, "wheelBoundaryPassed": wheel_boundary_passed, "dpiPassed": dpi_passed, "contrastPassed": contrast_passed, "focusIndicatorsPassed": focus_indicators_passed, "themeGeometryPassed": theme_geometry_passed, "passed": passed}, ensure_ascii=False, indent=2))
sys.exit(0 if passed else 1)
