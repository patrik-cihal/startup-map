# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Interactive 2D semantic map of Y Combinator startups. Three-stage pipeline: scrape -> embed -> visualize. Deployed to GitHub Pages at https://patrik-cihal.github.io/startup-map.

## Build & Run Commands

### Visualization — Desktop (search + map)
```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo r --release -p visualization
```
Desktop mode includes both Search and Map modes with local FastEmbed for vector search.

### Visualization — Web (map only)
```bash
dx serve --platform web          # Dev server
dx build --platform web --release # Production build
```
Web build only ships the Map view (no embedding/search). Config in `Dioxus.toml` sets base_path to "startup-map" for GitHub Pages.

### Embedding Pipeline (Rust)
```bash
cargo r --release -p embedding
```
Requires `.env` in `embedding/` with `OPENAI_API_KEY` (used for tagline normalization via GPT).

### Scraping (Python)
```bash
python scraping/scrape_links.py    # Extract YC company links
python scraping/scrape_details.py  # Fetch company details as JSON
```
Dependencies: selenium, beautifulsoup4, webdriver-manager.

## Architecture

**Rust workspace** (2024 edition) with two crates:
- `embedding/` — Async pipeline: normalize taglines (OpenAI), generate 384-dim embeddings (FastEmbed), reduce to 2D (PaCMAP), output `startups.json`
- `visualization/` — Dioxus 0.7 app with two platform targets:
  - **Desktop**: Search mode (list view with sort/filter, vector similarity search via FastEmbed) + Map mode
  - **Web/WASM**: Map mode only (no search, no embedding dependencies)

Dependencies are split by `target_arch` in Cargo.toml — desktop gets `tokio`, `fastembed`, `keyboard-types`; wasm gets `gloo-timers`.

**Python scraping** in `scraping/`:
- Two-stage: first scrape batch listing pages for links, then fetch individual company JSON pages

**Data flow**: scraping outputs -> embedding pipeline reads, embeds, reduces dimensions -> `visualization/assets/startups.json` consumed by app

## Key Implementation Details

- Visualization uses Dioxus signals for zoom/pan state with smooth animation (32ms async loop)
- Company filtering is zoom-level based: 8 thresholds mapping zoom ranges to minimum team_size (from 20000 down to 25)
- Search (desktop only) uses cosine similarity between query embedding (FastEmbed) and stored 384-dim f32 vectors
- Search results displayed as compact list view with sort by team size or similarity, logarithmic team size filter slider
- All startup data embedded directly in the binary via `include_str!`
- UI styled with Tailwind CSS v4, JetBrains Mono font, dark tactical/terminal theme with green (#00ffaa) accent
- `#[cfg(target_arch = "wasm32")]` / `#[cfg(not(...))]` used throughout to split web/desktop code paths
