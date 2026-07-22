# Changelog

All notable changes to glance are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow [SemVer](https://semver.org/).

## [Unreleased]

The full clean-room build — feature parity with `mdterm` v2.0.0 plus fixes for its four
weaknesses. Not yet published to crates.io (version still `0.0.0`).

### Added
- **Viewer core** — interactive TUI (alt-screen, raw mode, mouse, resize), all navigation keys, pipe mode, config (`~/.config/glance/config.toml`).
- **Rendering** — headings, emphasis, lists/tasks, blockquotes, GitHub callouts (`> [!NOTE]` — fixes mdterm weakness #4), tables, inline/fenced code; instant regex highlighting upgraded by background **syntect** (75 languages).
- **Interactivity** — search (`/`), TOC (`o`), fuzzy headings (`:`), link picker (`f`) + local-file nav, multi-file tabs, auto-reload, theme + line-number toggles, and a copy stack (OSC 52 + native fallbacks) that works over SSH/tmux/Wayland/X11 — fixes weakness #3.
- **Images** — half-block (any color terminal) + Kitty, local and remote (`http(s)`) sources fetched/decoded on a worker thread.
- **Differentiators** — streaming stdin (`llm | glance`) with auto-follow — fixes weakness #2; slide mode (`-s`); self-contained HTML export (`--export html`).
- **Ports** — inline `$…$` LaTeX → Unicode math, a colored JSON viewer (`glance data.json`), and mermaid flowcharts as Unicode box-art.

### Performance
- First paint <1 ms (7.9 KB doc) / ~13 ms (5k lines); 4.34 MB binary (2.1× smaller than mdterm). Heavy features stay off the first-paint path — fixes weakness #1.

[Unreleased]: https://github.com/sutharjay1/glance
