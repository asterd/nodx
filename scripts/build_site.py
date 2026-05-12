#!/usr/bin/env python3
"""Build the publishable docs site under `site/`.

This script does three small jobs:

1. Mirror the markdown pages from `docs/` and the root spec files into
   `site/pages/`. The routes used by the site (`guide/02-syntax-tour`,
   `reference/blocks`, `spec/NODX-RFC-0001`, …) all resolve to files
   here.
2. Rewrite relative markdown links so they continue to work inside the
   single-page app:
   - `../reference/foo.md`  -> `#/reference/foo`
   - `./bar.md#section`     -> `#/bar#section`
   - `../../crates/foo`     -> https URL pointing at the GitHub repo
3. Generate `site/data/search-index.json`, a flat list of pages with
   plain text bodies used by the in-browser search.
4. Mirror the static browser playground and the JavaScript package files
   it imports, so the same GitHub Pages artifact can host live examples.

The output of this script is committable. CI runs it on push to keep
GitHub Pages up to date, but you can run it locally to preview.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import sys
from pathlib import Path

ROOT       = Path(__file__).resolve().parent.parent
DOCS       = ROOT / "docs"
SITE       = ROOT / "site"
PAGES_DIR  = SITE / "pages"
DATA_DIR   = SITE / "data"

REPO_BASE_DEFAULT = "https://github.com/your-org/nodx"

# Map source files into site routes.
ROUTES: dict[str, Path] = {
    # guide
    "guide/index":           DOCS / "guide" / "index.md",
    "guide/01-quickstart":   DOCS / "guide" / "01-quickstart.md",
    "guide/02-syntax-tour":  DOCS / "guide" / "02-syntax-tour.md",
    "guide/03-authoring":    DOCS / "guide" / "03-authoring.md",
    "guide/04-packaging":    DOCS / "guide" / "04-packaging.md",
    "guide/05-cli":          DOCS / "guide" / "05-cli.md",
    # reference
    "reference/blocks":       DOCS / "reference" / "blocks.md",
    "reference/inline":       DOCS / "reference" / "inline.md",
    "reference/attributes":   DOCS / "reference" / "attributes.md",
    "reference/ast":          DOCS / "reference" / "ast.md",
    "reference/diagnostics":  DOCS / "reference" / "diagnostics.md",
    "reference/profiles":     DOCS / "reference" / "profiles.md",
    "reference/themes":       DOCS / "reference" / "themes.md",
    "reference/limits":       DOCS / "reference" / "limits.md",
    "reference/ncp":          DOCS / "reference" / "ncp.md",
    "reference/conformance":  DOCS / "reference" / "conformance.md",
    # cookbook
    "cookbook/index":         DOCS / "cookbook" / "index.md",
    # reference index
    "reference/index":        DOCS / "reference" / "index.md",
    # internals
    "internals/index":               DOCS / "internals" / "index.md",
    "internals/architecture":        DOCS / "internals" / "architecture.md",
    "internals/scalability":         DOCS / "internals" / "scalability.md",
    "internals/streaming-evolution": DOCS / "internals" / "streaming-evolution.md",
    "internals/security-model":      DOCS / "internals" / "security-model.md",
    # spec & root artifacts
    "spec/NODX-RFC-0001": ROOT / "NODX-RFC-0001.md",
    "spec/SECURITY":      ROOT / "SECURITY.md",
}


LINK_RE = re.compile(r'(?<!!)\[([^\]]+)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)')

# Routes are the keys of ROUTES. Reverse map source -> route for link rewriting.
def build_source_to_route() -> dict[Path, str]:
    return {src.resolve(): route for route, src in ROUTES.items()}


def rewrite_links(text: str, source_path: Path, source_to_route: dict[Path, str],
                  repo_base: str) -> str:
    def repl(match: re.Match[str]) -> str:
        label = match.group(1)
        href  = match.group(2)
        title = match.group(3) or ""

        # Absolute URLs are left alone.
        if re.match(r"^(https?:|mailto:|tel:|#)", href):
            new_href = href
        else:
            anchor = ""
            if "#" in href:
                href_path, anchor = href.split("#", 1)
            else:
                href_path = href

            target = (source_path.parent / href_path).resolve()
            # Directory link -> map to its index page if we have one.
            if target.is_dir():
                index_md = target / "index.md"
                if index_md.resolve() in source_to_route:
                    target = index_md.resolve()
                else:
                    readme_md = target / "README.md"
                    if readme_md.resolve() in source_to_route:
                        target = readme_md.resolve()
            if target in source_to_route:
                new_href = "#/" + source_to_route[target] + (("#" + anchor) if anchor else "")
            else:
                # Path inside the repo but not in our routes — link to GitHub.
                try:
                    rel = target.relative_to(ROOT)
                    new_href = f"{repo_base}/blob/main/{rel.as_posix()}"
                    if anchor:
                        new_href += "#" + anchor
                except ValueError:
                    new_href = href  # leave as-is

        title_suffix = f' "{title}"' if title else ""
        return f"[{label}]({new_href}{title_suffix})"

    return LINK_RE.sub(repl, text)


def strip_markdown(text: str) -> str:
    """Return a rough plain-text version of `text` for search snippets."""
    # Remove fenced code blocks completely.
    text = re.sub(r"```[\s\S]*?```", " ", text)
    # Inline code.
    text = re.sub(r"`([^`]+)`", r"\1", text)
    # Links: keep the label.
    text = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", text)
    # Headings.
    text = re.sub(r"^#{1,6}\s+", "", text, flags=re.MULTILINE)
    # Tables and pipes.
    text = re.sub(r"\|", " ", text)
    # Emphasis markers.
    text = re.sub(r"[*_]{1,3}", "", text)
    # Collapse whitespace.
    return " ".join(text.split())


def title_of(text: str, fallback: str) -> str:
    m = re.search(r"^#\s+(.+?)\s*$", text, flags=re.MULTILINE)
    return m.group(1).strip() if m else fallback


def main() -> int:
    repo_base = os.environ.get("NODX_REPO_BASE", REPO_BASE_DEFAULT).rstrip("/")
    src_to_route = build_source_to_route()

    if PAGES_DIR.exists():
        shutil.rmtree(PAGES_DIR)
    PAGES_DIR.mkdir(parents=True, exist_ok=True)
    DATA_DIR.mkdir(parents=True, exist_ok=True)

    search_index: list[dict[str, str]] = []

    for route, source in ROUTES.items():
        if not source.exists():
            print(f"!! source missing for route {route}: {source}", file=sys.stderr)
            return 1
        text = source.read_text(encoding="utf-8")
        rewritten = rewrite_links(text, source.resolve(), src_to_route, repo_base)

        dest = PAGES_DIR / (route + ".md")
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(rewritten, encoding="utf-8")

        plain = strip_markdown(rewritten)
        search_index.append({
            "route": route,
            "title": title_of(rewritten, route),
            "text":  plain[:1600],
        })
        print(f"-> {route} ({source.relative_to(ROOT)})")

    # Stable order = sidebar order = nice diff.
    DATA_DIR.joinpath("search-index.json").write_text(
        json.dumps(search_index, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    # Inject the configured repo base into a small JS file that
    # index.html loads ahead of router.js. Allows GitHub-link
    # rewriting without committing the URL.
    SITE.joinpath("assets", "config.js").write_text(
        "window.NODX_REPO_BASE = " + json.dumps(repo_base) + ";\n",
        encoding="utf-8",
    )

    mirror_static_dir(ROOT / "apps" / "web", SITE / "apps" / "web")
    mirror_static_dir(ROOT / "packages" / "nodx-js", SITE / "packages" / "nodx-js")
    mirror_static_dir(ROOT / "examples", SITE / "examples")
    ensure_playground_redirect()
    verify_publishable_site()

    print(f"\nWrote {len(ROUTES)} pages and a {len(search_index)}-entry search index.")
    print(f"Repo base for GitHub links: {repo_base}")
    return 0


def mirror_static_dir(source: Path, dest: Path) -> None:
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(
        source,
        dest,
        ignore=shutil.ignore_patterns("__pycache__", ".pytest_cache", "target", "node_modules"),
    )
    print(f"-> mirrored {source.relative_to(ROOT)} to {dest.relative_to(ROOT)}")


def ensure_playground_redirect() -> None:
    path = SITE / "playground" / "index.html"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="refresh" content="0; url=../apps/web/index.html">
<link rel="canonical" href="../apps/web/index.html">
<title>NODX Playground</title>
</head>
<body>
<p><a href="../apps/web/index.html">Open the NODX Playground</a></p>
</body>
</html>
""",
        encoding="utf-8",
    )
    print("-> playground redirect (site/playground/index.html)")


def verify_publishable_site() -> None:
    required = [
        SITE / "index.html",
        SITE / "playground" / "index.html",
        SITE / "apps" / "web" / "index.html",
        SITE / "apps" / "web" / "app.js",
        SITE / "apps" / "web" / "examples.js",
        SITE / "apps" / "web" / "styles.css",
        SITE / "packages" / "nodx-js" / "parser.mjs",
        SITE / "examples" / "playground" / "01-plain-document.nodx",
        SITE / "examples" / "playground" / "assets" / "remote-surface.nods",
    ]
    missing = [path for path in required if not path.is_file()]
    if missing:
        print("!! publishable site is missing required files:", file=sys.stderr)
        for path in missing:
            print(f"!! - {path.relative_to(ROOT)}", file=sys.stderr)
        raise SystemExit(1)
    print("-> verified publishable playground files")


if __name__ == "__main__":
    sys.exit(main())
