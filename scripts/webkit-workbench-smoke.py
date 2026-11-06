#!/usr/bin/python3
"""Synthetic WebKitGTK DOM checks against existing dist; not native Tauri IPC.

Run after the parent builds the UI: /usr/bin/python3 scripts/webkit-workbench-smoke.py
Requires a graphical session and system Python gi (Gtk 3 / WebKit2 4.1).
Pointer, wheel, and HTML drag events are dispatched DOM events, not physical input.
Only discovery is mocked; the browser fallback uses a fresh, unmocked WebView.
"""

from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import sys
import threading

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import GLib, Gtk, WebKit2  # noqa: E402


ROOT = Path(__file__).resolve().parent.parent
TIMEOUT_SECONDS = 30


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        pass


class Smoke:
    def __init__(self, uri, checks, catalog):
        self.uri = uri
        self.checks = checks
        self.catalog = catalog
        self.window = Gtk.Window(title="Nodus synthetic WebKit DOM smoke")
        self.window.set_default_size(1280, 800)
        self.window.connect("delete-event", self.closed)
        self.view = None
        self.manager = None
        self.context = None
        self.mode = None
        self.results = []
        self.finished = False
        self.timeout_id = None

    def closed(self, *_args):
        self.abort("Smoke window closed before completion")
        return True

    def dispose_view(self):
        if self.view is not None:
            view = self.view
            self.view = None
            view.stop_loading()
            view.terminate_web_process()
            self.window.remove(view)
            view.destroy()
        if self.manager is not None:
            self.manager.unregister_script_message_handler("nodusSmoke")
            self.manager = None
        self.context = None

    def start_page(self, mode):
        self.dispose_view()
        self.mode = mode
        self.manager = WebKit2.UserContentManager()
        self.manager.connect("script-message-received::nodusSmoke", self.message)
        if not self.manager.register_script_message_handler("nodusSmoke"):
            self.abort("Could not register DOM result message handler")
            return GLib.SOURCE_REMOVE
        if mode == "mocked-discovery":
            boundary = """
                globalThis.isTauri = true;
                globalThis.__TAURI_INTERNALS__ = {
                    invoke: async (command) => {
                        if (command !== 'list_components') {
                            throw new Error('Unexpected mocked IPC command: ' + command);
                        }
                        return CATALOG;
                    }
                };
            """.replace("CATALOG", json.dumps(self.catalog))
            self.manager.add_script(WebKit2.UserScript.new(
                boundary, WebKit2.UserContentInjectedFrames.TOP_FRAME,
                WebKit2.UserScriptInjectionTime.START, None, None,
            ))
        self.context = WebKit2.WebContext.new_ephemeral()
        self.view = WebKit2.WebView(
            web_context=self.context, user_content_manager=self.manager,
        )
        self.view.connect("load-changed", self.loaded)
        self.view.connect("load-failed", self.load_failed)
        self.view.connect("web-process-terminated", self.process_terminated)
        self.window.add(self.view)
        self.window.show_all()
        self.view.load_uri(self.uri)
        return GLib.SOURCE_REMOVE

    def loaded(self, view, event):
        if event != WebKit2.LoadEvent.FINISHED or view != self.view or self.finished:
            return
        options = {"mode": self.mode, "fixtureSvg": self.catalog["packages"][0]["svg"]}
        source = "globalThis.__nodusSmokeOptions = " + json.dumps(options) + ";\n" + self.checks
        view.evaluate_javascript(source, -1, None, self.uri, None, self.evaluated, None)

    def evaluated(self, view, result, _data):
        try:
            view.evaluate_javascript_finish(result)
        except GLib.Error as error:
            if view == self.view and not self.finished:
                self.abort(f"JavaScript evaluation failed: {error.message}")

    def load_failed(self, view, _event, uri, error):
        if view == self.view and not self.finished:
            self.abort(f"Page load failed at {uri}: {error.message}")
        return False

    def process_terminated(self, view, reason):
        if view == self.view and not self.finished:
            self.abort(f"WebKit web process terminated unexpectedly: {reason}")

    def message(self, manager, message):
        if manager != self.manager or self.finished:
            return
        try:
            result = json.loads(message.get_js_value().to_string())
            if (result.get("mode") != self.mode or type(result.get("ok")) is not bool
                    or not isinstance(result.get("passed"), list)):
                raise ValueError("Invalid DOM result envelope")
        except (ValueError, TypeError, AttributeError) as error:
            self.abort(f"Invalid DOM result: {error}")
            return
        self.results.append(result)
        print(json.dumps(result, ensure_ascii=False), flush=True)
        if self.mode == "mocked-discovery":
            # Leave the message callback before destroying its WebView.
            GLib.idle_add(self.start_page, "browser-fallback")
        else:
            self.finished = True
            Gtk.main_quit()

    def abort(self, error):
        if self.finished:
            return GLib.SOURCE_REMOVE
        self.results.append({"mode": self.mode, "ok": False, "error": error})
        print(f"FAIL: {error}", file=sys.stderr, flush=True)
        self.finished = True
        Gtk.main_quit()
        return GLib.SOURCE_REMOVE

    def timed_out(self):
        self.timeout_id = None
        return self.abort(f"Shared {TIMEOUT_SECONDS}s deadline exceeded ({self.mode})")

    def run(self):
        self.timeout_id = GLib.timeout_add_seconds(TIMEOUT_SECONDS, self.timed_out)
        try:
            self.start_page("mocked-discovery")
            if not self.finished:
                Gtk.main()
            return (len(self.results) == 2
                    and all(result.get("ok") for result in self.results))
        finally:
            self.finished = True
            if self.timeout_id is not None:
                GLib.source_remove(self.timeout_id)
            self.dispose_view()
            self.window.destroy()


def main():
    dist = ROOT / "dist"
    if not (dist / "index.html").is_file():
        raise RuntimeError("dist/index.html missing; build the intended UI first")
    checks = (ROOT / "scripts/workbench-dom-checks.js").read_text(encoding="utf-8")
    catalog = {"packages": [{
        "manifest": json.loads((ROOT / "docs/nodpkg-v1-led.json").read_text(encoding="utf-8")),
        "svg": (ROOT / "components-registry/basic-led.nodpkg/graphic.svg").read_text(encoding="utf-8"),
    }], "diagnostics": []}
    if not Gtk.init_check()[0]:
        raise RuntimeError("GTK display unavailable; run inside a graphical session")
    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(QuietHandler, directory=str(dist)))
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.1}, daemon=True)
    try:
        thread.start()
        uri = f"http://127.0.0.1:{server.server_port}/"
        print("Synthetic WebKit DOM smoke: mocked discovery; no native Tauri IPC or physical pointer input.", flush=True)
        print(f"Serving existing dist at {uri}; shared deadline {TIMEOUT_SECONDS}s", flush=True)
        passed = Smoke(uri, checks, catalog).run()
        print("PASS" if passed else "FAIL", flush=True)
        return 0 if passed else 1
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (RuntimeError, OSError, GLib.Error) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
