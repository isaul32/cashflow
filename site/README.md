# CashFlow website

Static site for CashFlow, built with [Astro](https://astro.build). Six languages
(`/`, `/fi/`, `/sv/`, `/de/`, `/fr/`, `/es/`), light and dark theme, OS detection for the
download button.

```bash
npm install
npm run dev       # http://localhost:4321
npm run build     # output in dist/
```

## Structure

- `src/i18n/translations.ts`: all texts, one entry per language. A missing key fails the type check.
- `src/components/Landing.astro`: the page; `src/layouts/Base.astro`: head, SEO tags, hreflang, JSON-LD.
- `public/_redirects`: stable `/download/*` links. Point them at release assets once the release
  workflow publishes version-less file names.
- `public/_headers`: security and cache headers. `tools/make_og.py` regenerates `public/og.png`.
- Screenshots live in `src/assets/screenshots/<lang>-<light|dark>.png` and are converted to WebP at build time.

## Cloudflare Pages

| Setting | Value |
| --- | --- |
| Root directory | `site` |
| Build command | `npm run build` |
| Build output directory | `dist` |
| `SITE_URL` (environment variable) | the final site URL, e.g. `https://cashflow.example.com` |

`SITE_URL` is used for canonical URLs, hreflang links, the sitemap and the social image.

## Screenshots

The images are real captures of the app, one per language and theme, made from the demo data in
`tools/demo/<lang>/` (translated category and entry names for each language):

```powershell
powershell -ExecutionPolicy Bypass -File site/tools/screenshots.ps1
```

The script builds the release app, runs it with `CASHFLOW_MONTH` and `CASHFLOW_THEME` set, and copies
the window pixels, so it needs a visible Windows desktop and takes about three minutes. It also
refreshes `docs/screenshot.png` for the README. Options: `-Languages fi,en`, `-Themes light`, `-SkipBuild`.

A git pre-commit hook runs it automatically when a commit changes the app (`src/`, `assets/`,
`Cargo.*`) or the demo data, and stages the new images. Enable it once per clone:

```bash
git config core.hooksPath .githooks
```

Skip it for a single commit with `SKIP_SCREENSHOTS=1 git commit ...`. The hook captures the working
tree, not only the staged changes.
