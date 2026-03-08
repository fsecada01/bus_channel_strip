# Documentation Site Enhancement Plan

**Repo**: `fsecada01/bus_channel_strip`
**Framework**: Astro 5 + Starlight 0.32
**Current state**: 12 pages deployed to `https://fsecada01.github.io/bus_channel_strip/`
**Branch**: `claude/fix-docs-generation-J3uVs`

---

## Phase 1 — CI/CD & Build Hygiene

### 1.1 Auto-deploy docs on push to `main`

**Problem**: `pages.yml` only triggers on `release: [published]` and `workflow_dispatch`. Documentation changes merged to `main` don't auto-deploy.

**Fix**: Add a `push` trigger filtered to docs-related paths.

```yaml
# .github/workflows/pages.yml
on:
  push:
    branches: [main]
    paths:
      - 'site/**'
      - 'docs/**'
      - 'README.md'
  release:
    types: [published]
  workflow_dispatch:
```

**Files**: `.github/workflows/pages.yml`

### 1.2 Remove committed build artifacts

**Problem**: `site/dist/` (the Astro build output) is checked into git. This bloats the repo and causes merge conflicts.

**Fix**:
1. Add `site/dist/` to `.gitignore` (verify it isn't already covered)
2. Remove tracked files: `git rm -r --cached site/dist/`
3. CI builds fresh on every deploy — no need to commit artifacts

**Files**: `.gitignore`, `site/dist/` (removal)

### 1.3 Add `just docs` recipes

**Fix**: Add documentation dev/build/preview commands to the justfile (if one exists) or document them clearly.

```just
# Local docs development server
docs-dev:
    cd site && npm run dev

# Build docs for production
docs-build:
    cd site && npm install && npm run build

# Preview built docs locally
docs-preview:
    cd site && npm run preview
```

**Files**: `justfile` (or document in `CLAUDE.md`)

---

## Phase 2 — Missing Content Pages

### 2.1 Contributing / Development Guide

**Path**: `site/src/content/docs/contributing.md`

**Content outline**:
- Prerequisites (Rust nightly, LLVM, Node 20)
- Clone and build instructions (plugin + docs site)
- Running tests and quality gate (`cargo test`, `cargo clippy`, `cargo fmt`)
- Feature flag reference
- PR workflow and branch conventions
- Audio thread rules summary (no alloc, no lock, no panic, no I/O)

**Sidebar addition**: Top-level "Contributing" entry after "Presets & Techniques"

### 2.2 Architecture / Internals page

**Path**: `site/src/content/docs/architecture.md`

**Content outline**:
- Plugin architecture diagram (NIH-Plug → Params → Process → GUI)
- Module reordering system explanation
- Parameter ID scheme and stability policy
- FFI boundary (ButterComp2 C++ wrapper)
- Lock-free audio thread design
- Feature flag topology and how modules are conditionally compiled

**Sidebar addition**: Top-level "Architecture" entry

### 2.3 Changelog page

**Path**: `site/src/content/docs/changelog.md`

**Content outline**:
- v0.3.0: Astro docs site, CI fixes, macOS Intel headless
- v0.2.0: Punch module, drag-to-swap UI, vizia GUI
- v0.1.x: Initial modules, FFI integration

Could also be auto-generated from git tags/releases using a build script.

### 2.4 Full Parameter Reference

**Path**: `site/src/content/docs/parameters.md`

**Content outline**:
- Complete table of all ~75+ parameters with: name, ID, range, default, module, automation support
- Generated or manually maintained from `src/lib.rs` `Params` struct
- Useful for preset authors and DAW automation users

**Sidebar addition**: Under a new "Reference" section

---

## Phase 3 — Starlight Feature Improvements

### 3.1 Enable "Edit this page" links

Starlight supports linking each page back to its source on GitHub for community contributions.

```js
// astro.config.mjs — inside starlight({})
editLink: {
  baseUrl: 'https://github.com/fsecada01/bus_channel_strip/edit/main/site/',
},
```

**Files**: `site/astro.config.mjs`

### 3.2 Add favicon and OpenGraph image

**Problem**: No custom favicon or social sharing image.

**Fix**:
1. Add a plugin icon/logo as `site/public/favicon.svg`
2. Add an OG image as `site/public/og-image.png` (1200×630)
3. Configure in Starlight:

```js
// astro.config.mjs
head: [
  { tag: 'meta', attrs: { property: 'og:image', content: '/bus_channel_strip/og-image.png' } },
],
favicon: '/favicon.svg',
```

**Files**: `site/astro.config.mjs`, `site/public/favicon.svg`, `site/public/og-image.png`

### 3.3 Add search configuration

Starlight bundles [Pagefind](https://pagefind.app/) for search. Verify it's working and consider adding:

```js
// astro.config.mjs
pagefind: true, // should be default, but make explicit
```

**Files**: `site/astro.config.mjs`

### 3.4 Add last-updated timestamps

Show when each page was last modified using git history.

```js
// astro.config.mjs
lastUpdated: true,
```

**Files**: `site/astro.config.mjs`

---

## Phase 4 — Content Quality & Visual Polish

### 4.1 Add GUI screenshots

**Problem**: No visual documentation of the plugin interface.

**Fix**:
1. Capture screenshots of the vizia GUI in a DAW (Reaper preferred)
2. Add to `site/src/assets/` (Astro optimizes images in this dir)
3. Embed in landing page, module pages, and installation guide
4. Key screenshots needed:
   - Full plugin view (default module order)
   - Each module section close-up
   - Drag-to-swap reordering in action
   - Spectral analyzer / GR metering on Dynamic EQ

**Files**: `site/src/assets/screenshots/`, multiple `.md`/`.mdx` pages

### 4.2 Add signal flow diagrams

**Problem**: Signal chain is shown as styled `<span>` elements — looks nice but could be enhanced with actual diagrams showing internal routing.

**Options**:
- Use [Mermaid](https://github.com/withastro/starlight/tree/main/packages/starlight-mermaid) via `starlight-mermaid` integration
- Or keep the current CSS approach and add per-module internal flow diagrams as SVGs

### 4.3 Improve module pages with "Quick Start" sections

Each module page has Controls and Techniques but could benefit from a **Quick Start** box at the top:

```md
:::tip[Quick Start]
For a warm, punchy drum bus: set Compress to 0.65, Output to 0.85, Dry/Wet to 0.40. Adjust to taste.
:::
```

**Files**: All 6 module `.md`/`.mdx` files

### 4.4 Cross-link between module pages and preset pages

**Problem**: Module pages mention techniques but don't link to the Presets section, and vice versa.

**Fix**: Add "See also" sections at the bottom of each module page linking to relevant preset recipes, and add module links in preset pages.

---

## Phase 5 — Advanced Enhancements (Lower Priority)

### 5.1 Auto-generate parameter reference from source

Write a build-time script (Node or Rust) that parses `src/lib.rs` and extracts all `#[id = "..."]` parameter definitions, ranges, and defaults into a JSON file. Astro can then render this as a searchable table.

**Approach**:
1. Add a `scripts/extract-params.rs` or `scripts/extract-params.mjs`
2. Output to `site/src/data/parameters.json`
3. Create an Astro component that renders the JSON as a filterable table
4. Run extraction as a pre-build step in `pages.yml`

### 5.2 Version-gated documentation

If the plugin ships multiple versions with breaking parameter changes, consider Starlight's sidebar badges or a version selector. Not needed until v1.0.

### 5.3 Audio examples (embedded players)

Embed before/after audio clips demonstrating each module's effect. Use `<audio>` elements with `.wav` or `.mp3` files in `site/public/audio/`.

### 5.4 PDF export of documentation

For offline reference, add a Starlight plugin or post-build step that generates a PDF of the full docs. Low priority.

---

## Implementation Order (Recommended)

| Priority | Task | Effort | Impact |
|----------|------|--------|--------|
| **P0** | 1.1 Auto-deploy on push | 5 min | High — docs stay current |
| **P0** | 1.2 Remove committed dist/ | 5 min | High — repo hygiene |
| **P1** | 3.1 Edit-this-page links | 2 min | Medium — contributor UX |
| **P1** | 3.4 Last-updated timestamps | 2 min | Medium — trust signal |
| **P1** | 4.4 Cross-links | 20 min | Medium — navigation |
| **P2** | 2.1 Contributing guide | 45 min | Medium — onboarding |
| **P2** | 2.2 Architecture page | 60 min | Medium — developer docs |
| **P2** | 4.1 GUI screenshots | 30 min | High — visual appeal |
| **P2** | 4.3 Quick Start boxes | 30 min | Medium — user experience |
| **P2** | 1.3 Just recipes | 10 min | Low — DX convenience |
| **P3** | 2.3 Changelog | 20 min | Low — already on GitHub releases |
| **P3** | 2.4 Parameter reference | 60 min | Medium — automation users |
| **P3** | 3.2 Favicon + OG image | 30 min | Low — polish |
| **P3** | 4.2 Signal flow diagrams | 45 min | Low — visual polish |
| **P4** | 5.1 Auto-gen params | 3 hrs | Medium — maintenance reduction |
| **P4** | 5.3 Audio examples | 2 hrs | High — but requires audio assets |

---

## Files Modified Per Phase

| Phase | Files touched |
|-------|---------------|
| 1 | `.github/workflows/pages.yml`, `.gitignore`, `justfile` |
| 2 | `site/astro.config.mjs`, 4 new `.md` files in `site/src/content/docs/` |
| 3 | `site/astro.config.mjs`, `site/public/` (new assets) |
| 4 | All existing module/preset `.md`/`.mdx` files, `site/src/assets/` |
| 5 | `scripts/`, `site/src/data/`, `site/public/audio/` |

---

## How to Use This Plan

Feed this file to your Claude instance with the following instruction:

> Implement the documentation enhancements described in `docs/DOCS_ENHANCEMENT_PLAN.md` for the bus_channel_strip repository. Start with Phase 1 (P0 items), then proceed through each phase in priority order. Commit each phase as a separate commit. The docs site uses Astro 5 + Starlight in the `site/` directory. The CI workflow is at `.github/workflows/pages.yml`.
