# NODX docs site

The static, publishable documentation site for NODX. Plain HTML/CSS/JS,
no build dependencies beyond Python 3.

## Layout

```
site/
├─ index.html            ← shell, navigation, theme/search wiring
├─ favicon.svg
├─ assets/
│  ├─ site.css           ← single hand-written stylesheet
│  ├─ markdown.js        ← tiny Markdown renderer (no CDN)
│  ├─ router.js          ← hash-based SPA, sidebar/outline/search
│  └─ config.js          ← injected by build_site.py (repo URL)
├─ pages/                ← generated; mirrors docs/ as flat .md files
└─ data/
   └─ search-index.json  ← generated; powers the sidebar search box
```

## Build locally

```sh
python3 scripts/build_site.py
python3 -m http.server 8765 -d site
# open http://127.0.0.1:8765/
```

`scripts/build_site.py` mirrors `docs/**` into `site/pages/`, rewrites
relative Markdown links into SPA hash routes, and generates the search
index.

## Deploy

GitHub Pages is wired through `.github/workflows/docs.yml`. The workflow:

1. Runs `python3 scripts/build_site.py`.
2. Uploads `site/` as a Pages artifact.
3. Publishes it to the configured Pages environment.

Enable Pages once in the repository settings (Source: *GitHub Actions*).
After that the workflow handles every push to `main` that touches
`docs/`, `site/`, the build script, or the spec/security docs.

## Browser support

Designed for any browser shipped in the last two years. No build step
means no transpilation; the JavaScript uses standard features only.
Light/dark themes follow `prefers-color-scheme` with a manual toggle
that persists to `localStorage`.

## Editing pages

Edit Markdown under `docs/`. The site picks the files up on the next
`build_site.py` run. The routes the site exposes are listed in the
`ROUTES` map at the top of `scripts/build_site.py`; add or remove entries
there to change the sidebar.
