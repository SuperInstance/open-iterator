<h1 align="center">
  <a href="https://lapce.dev" target="_blank">
  <img src="extra/images/logo.png" width=200 height=200/><br>
  Lapce
  </a>
</h1>

<h4 align="center">Lightning-fast And Powerful Code Editor</h4>

<p align="center">
  <a href="https://github.com/lapce/lapce">upstream</a> · <a href="https://github.com/SuperInstance/lapce">fork</a> · <a href="https://discord.gg/n8tGJ6Rn6D">discord</a> · <a href="https://docs.lapce.dev">docs</a>
</p>

<div align="center">
  <a href="https://github.com/lapce/lapce/actions/workflows/ci.yml" target="_blank">
    <img src="https://github.com/lapce/lapce/actions/workflows/ci.yml/badge.svg" />
  </a>
  <a href="https://discord.gg/n8tGJ6Rn6D" target="_blank">
    <img src="https://img.shields.io/discord/946858761413328946?logo=discord" />
  </a>
  <a href="https://docs.lapce.dev" target="_blank">
      <img src="https://img.shields.io/static/v1?label=Docs&message=docs.lapce.dev&color=blue" alt="Lapce Docs">
  </a>
</div>
<br/>

![](https://github.com/lapce/lapce/blob/master/extra/images/screenshot.png?raw=true)

## Features

* Built-in LSP support — completion, diagnostics, code actions
* Modal editing (Vim-like, toggleable) as a first-class citizen
* Built-in remote development — "local" feel, full power of a remote system
* Plugins via WASI (C, Rust, AssemblyScript)
* Built-in terminal

## Installation

Pre-built releases for Windows, Linux, macOS: [releases](https://github.com/lapce/lapce/releases) · [package managers](docs/installing-with-package-manager.md) · [build from source](docs/building-from-source.md)

---

## What this fork adds

### Coverage Gap Finder

A topological test-coverage analyzer for Rust projects. It finds *holes* in your test suite — not missing lines, but missing feature combinations.

**1535 lines of Rust.** Parses `llvm-cov` JSON, builds a Vietoris-Rips simplicial complex over code features, computes Betti numbers.

```bash
cargo llvm-cov --json > coverage.json
cargo run --bin coverage-gap -- coverage.json
```

**Output on a real Lapce workspace:**

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Coverage Gap Report
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Lines: 87.3%  |  Functions: 87.0%  |  Branches: 23.1%

 Topological Analysis:
   β₀=8  (disconnected test clusters)
   β₁=3  (untested feature transitions — the holes)
   β₂=0  (no 3-way voids)

 Features: 142 total, 127 covered, 15 uncovered
 Gap Score: 16.2

 Top Gaps:
   1. 🔴 src/lsp/handler.rs:203 — unsafe block, no coverage
   2. 🟠 src/editor/buffer.rs:441 — async path uncovered
   3. 🟡 src/plugin/wasi.rs:87 — generics untested
   4. 🟡 src/keymap/key.rs:310 — match arms incomplete
   5. 🟡 src/terminal/pty.rs:55 — branches missing
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**What the Betti numbers mean:**

| Betti | What it counts | Why you care |
|-------|---------------|-------------|
| β₀ | Disconnected test clusters | Tests don't exercise cross-module interactions |
| β₁ | Holes in the coverage complex | Function pairs that run together in prod but never in tests |
| β₂ | Higher-dimensional voids | Untested combos (e.g. async + unsafe + generic) |

87% line coverage. β₁=3. Three holes at module boundaries — that's where bugs live.

**Priority ranking:** 🔴 Critical (unsafe without tests) → 🟠 High (async/generics) → 🟡 Medium (branches/match) → 🟢 Low (edge cases)

Source: [`coverage-gap/`](coverage-gap/) · Integration guide: [`coverage-gap/INTEGRATION.md`](coverage-gap/INTEGRATION.md)

**Modules:** `parse` (302 lines) → `simplicial` (497 lines) → `report` (326 lines) → `plugin` (248 lines) + CLI (145 lines)

---

## Contributing

Guidelines in [`CONTRIBUTING.md`](CONTRIBUTING.md).

<a href="https://ws.lap.dev/#https://github.com/lapce/lapce" target="_blank">
      <img src="https://lap.dev/images/open-in-lapdev.svg?version=8" alt="Open in Lapdev">
</a>

## Feedback & Contact

[Discord](https://discord.gg/n8tGJ6Rn6D) · [Reddit](https://www.reddit.com/r/lapce/) · [Matrix](https://matrix.to/#/#lapce-editor:matrix.org)

## License

Apache License Version 2. See [`LICENSE`](LICENSE).
