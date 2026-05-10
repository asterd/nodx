#!/usr/bin/env python3
"""Dependency-free desktop-style viewer for NODX documents.

It builds the Rust CLI if needed, renders the document to safe HTML, serves a
local read-only page, and opens the default browser. This avoids Tkinter, which
is not available in every Python distribution.
"""

from __future__ import annotations

import html
import http.server
import json
import socketserver
import subprocess
import sys
import threading
import urllib.parse
import webbrowser
from pathlib import Path


class ViewerHandler(http.server.BaseHTTPRequestHandler):
    page = ""
    root_dir = Path(".")

    def do_GET(self) -> None:
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path.startswith("/examples/"):
            target = (self.root_dir / parsed.path.lstrip("/")).resolve()
            try:
                target.relative_to(self.root_dir)
            except ValueError:
                self.send_error(403)
                return
            if target.is_file():
                content_type = "image/svg+xml" if target.suffix == ".svg" else "text/plain; charset=utf-8"
                self.send_response(200)
                self.send_header("Content-Type", content_type)
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                self.wfile.write(target.read_bytes())
                return

        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(self.page.encode("utf-8"))

    def log_message(self, _fmt: str, *_args: object) -> None:
        return


def build_page(title: str, rendered_html: str, tui: str, ast: str) -> str:
    payload = json.dumps({"tui": tui, "ast": ast})
    return f"""<!doctype html>
<meta charset="utf-8">
<title>NODX Viewer - {html.escape(title)}</title>
<style>
  :root{{color-scheme:light dark;--bg:#f7f7f4;--panel:#fff;--ink:#1f2528;--muted:#66717a;--line:#d7d8d2;--accent:#0f766e;--warn:#b45309}}
  @media (prefers-color-scheme:dark){{:root{{--bg:#111315;--panel:#181b1f;--ink:#e8ecef;--muted:#9aa4ad;--line:#30363d;--accent:#2dd4bf;--warn:#f59e0b}}}}
  *{{box-sizing:border-box}}
  body{{margin:0;background:var(--bg);color:var(--ink);font:14px system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}}
  header{{height:52px;display:flex;align-items:center;justify-content:space-between;padding:0 18px;border-bottom:1px solid var(--line);background:var(--panel)}}
  header strong{{font-size:15px}}
  nav button{{border:1px solid var(--line);background:transparent;color:var(--ink);padding:6px 10px;margin-left:6px;border-radius:6px;cursor:pointer}}
  nav button[aria-pressed="true"]{{border-color:var(--accent);color:var(--accent)}}
  main{{height:calc(100vh - 52px);overflow:auto;padding:28px}}
  .doc{{max-width:980px;margin:0 auto;background:var(--panel);border:1px solid var(--line);padding:32px;border-radius:8px}}
  .hidden{{display:none}}
  pre{{white-space:pre-wrap;background:#0f172a;color:#dbeafe;padding:16px;border-radius:8px;overflow:auto}}
  code{{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}}
  table{{border-collapse:collapse;width:100%;margin:16px 0}}td,th{{border:1px solid var(--line);padding:8px 10px;text-align:left}}th{{background:color-mix(in srgb,var(--accent),transparent 88%)}}
  aside{{border-left:4px solid var(--warn);padding:10px 14px;background:color-mix(in srgb,var(--warn),transparent 90%);margin:16px 0}}
  figure{{margin:18px 0;padding:14px;border:1px solid var(--line);border-radius:8px}}figcaption{{color:var(--muted);font-size:13px;margin-top:8px}}
</style>
<header>
  <strong>{html.escape(title)}</strong>
  <nav>
    <button data-tab="rendered" aria-pressed="true">Rendered</button>
    <button data-tab="tui" aria-pressed="false">TUI</button>
    <button data-tab="ast" aria-pressed="false">AST</button>
  </nav>
</header>
<main>
  <section id="rendered" class="doc">{rendered_html}</section>
  <section id="tui" class="doc hidden"><pre id="tui-pre"></pre></section>
  <section id="ast" class="doc hidden"><pre id="ast-pre"></pre></section>
</main>
<script>
const payload = {payload};
document.getElementById("tui-pre").textContent = payload.tui.replace(/\\x1b\\[[0-9;]*m/g, "");
document.getElementById("ast-pre").textContent = payload.ast;
document.querySelectorAll("button[data-tab]").forEach((button) => {{
  button.addEventListener("click", () => {{
    document.querySelectorAll("button[data-tab]").forEach((b) => b.setAttribute("aria-pressed", String(b === button)));
    document.querySelectorAll("main > section").forEach((s) => s.classList.toggle("hidden", s.id !== button.dataset.tab));
  }});
}});
</script>
"""


def main() -> int:
    if len(sys.argv) not in (2, 3) or (len(sys.argv) == 3 and sys.argv[1] != "--check"):
        print("usage: nodx_viewer.py [--check] <file.nodx>", file=sys.stderr)
        return 2

    root_dir = Path(__file__).resolve().parents[2]
    check_only = len(sys.argv) == 3
    path = Path(sys.argv[-1]).resolve()
    nodx = root_dir / "target" / "debug" / "nodx"
    if not nodx.exists():
        subprocess.run(["cargo", "build", "-q", "-p", "nodx"], cwd=root_dir, check=True)

    rendered = subprocess.check_output([str(nodx), "html", str(path)], text=True)
    tui = subprocess.check_output([str(nodx), "tui", str(path)], text=True)
    ast = subprocess.check_output([str(nodx), "ast", str(path)], text=True)

    ViewerHandler.page = build_page(path.name, rendered, tui, ast)
    ViewerHandler.root_dir = root_dir.resolve()
    if check_only:
        print(f"NODX viewer check passed for {path}")
        return 0

    with socketserver.TCPServer(("127.0.0.1", 0), ViewerHandler) as server:
        port = server.server_address[1]
        url = f"http://127.0.0.1:{port}/"
        threading.Timer(0.2, lambda: webbrowser.open(url)).start()
        print(f"NODX viewer running at {url}")
        print("Press Ctrl-C to stop.")
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            return 0


if __name__ == "__main__":
    raise SystemExit(main())
