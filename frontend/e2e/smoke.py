"""Window smoke test: drives the running window in Chromium under 4x CPU throttling and checks that the main flows work
with no unexpected layout shift and no console errors.

Needs the service and the dev server running (./formsculpt boot) and Python Playwright with Chromium.
Run: ./formsculpt test e2e   (or python3 frontend/e2e/smoke.py [window url] [service url])
"""

import json
import sys
import tempfile
import urllib.request

from playwright.sync_api import sync_playwright

WINDOW = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:5260/"
SERVICE = sys.argv[2] if len(sys.argv) > 2 else "http://127.0.0.1:8260"
FAILURES: list[str] = []


def call(name: str, args: dict | None = None) -> dict:
    req = urllib.request.Request(f"{SERVICE}/api/engine/dispatch", data=json.dumps({"name": name, "args": args or {}}).encode(), headers={"content-type": "application/json"})
    return json.load(urllib.request.urlopen(req))


def objects() -> list:
    return call("get_scene")["result"]["objects"]


def check(label: str, ok: bool, detail: str = "") -> None:
    print(("ok    " if ok else "FAIL  ") + label + (f": {detail}" if detail else ""))
    if not ok:
        FAILURES.append(label)


def main() -> int:
    call("new_scene")
    with sync_playwright() as p:
        browser = p.chromium.launch(args=["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
        page = browser.new_context(viewport={"width": 1280, "height": 1000}, accept_downloads=True).new_page()
        errors: list[str] = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.on("console", lambda m: m.type == "error" and errors.append(m.text))
        page.context.new_cdp_session(page).send("Emulation.setCPUThrottlingRate", {"rate": 4})
        page.add_init_script("window.__cls=0;new PerformanceObserver(l=>{for(const e of l.getEntries())if(!e.hadRecentInput)window.__cls+=e.value}).observe({type:'layout-shift',buffered:true});")
        # The smoke test drives the panel layout's controls; the tablet layout is the default since 0.75.1, so the
        # panel layout is chosen the way a person's choice is kept.
        page.add_init_script("try{localStorage.setItem('formsculpt.tablet.default','1');localStorage.setItem('formsculpt.tablet','off')}catch(e){}")
        page.goto(WINDOW)
        page.wait_for_timeout(3000)
        panel = page.locator(".panel")
        btn = lambda name: panel.get_by_role("button", name=name, exact=True)
        note = lambda: page.locator(".panel-note").inner_text()
        canvas = page.locator("canvas.viewport")
        box = canvas.bounding_box()
        cx, cy = box["x"] + box["width"] / 2, box["y"] + box["height"] / 2

        def drag(points):
            page.mouse.move(*points[0])
            page.mouse.down()
            for q in points[1:]:
                page.mouse.move(*q, steps=6)
            page.mouse.up()

        check("the service answers", page.locator(".engine-state").inner_text().strip() == "service connected")
        btn("Sphere").click()
        page.wait_for_timeout(1500)
        check("a sphere is added", len(objects()) == 1)
        canvas.focus()
        page.keyboard.press("1")
        page.keyboard.press("f")
        page.wait_for_timeout(400)

        rev = objects()[0]["revision"]
        # new_scene is itself an undo step, so the history may hold earlier work; compare against where it stood.
        done = call("get_history")["result"]["done"]
        btn("Draw").click()
        drag([(cx - 80, cy), (cx + 80, cy)])
        page.wait_for_timeout(2000)
        check("a stroke changes the mesh", objects()[0]["revision"] > rev)
        btn("Undo").click()
        page.wait_for_timeout(1000)
        check("undo takes the stroke back", call("get_history")["result"]["done"] == done)

        btn("Lasso cut").click()
        drag([(cx - 50, cy - 50), (cx + 50, cy - 50), (cx + 50, cy + 50), (cx - 50, cy + 50), (cx - 50, cy - 45)])
        page.wait_for_timeout(6000)
        hit = call("pick", {"origin": [0, 0, 5], "direction": [0, 0, -1]})["result"]["hit"]
        check("a lasso cut goes through", hit is False, note())

        page.locator('input[aria-label="Text"]').fill("OK")
        btn("Build text").click()
        page.wait_for_timeout(2500)
        check("text is built", objects()[-1]["name"] == "OK", note())

        head = panel.locator("h2.fold").first
        head.click()
        page.wait_for_timeout(200)
        check("a section folds", head.get_attribute("aria-expanded") == "false")
        head.click()

        with page.expect_download(timeout=120000) as dl:
            btn("History movie").click()
        path = tempfile.mktemp(suffix=".png")
        dl.value.save_as(path)
        with open(path, "rb") as f:
            check("the history movie downloads", f.read(8) == b"\x89PNG\r\n\x1a\n")

        cls = page.evaluate("window.__cls")
        check("no unexpected layout shift", cls == 0, str(cls))
        check("no console errors", not errors, "; ".join(errors))
        browser.close()
    print(f"{len(FAILURES)} failed" if FAILURES else "all passed")
    return 1 if FAILURES else 0


if __name__ == "__main__":
    sys.exit(main())
