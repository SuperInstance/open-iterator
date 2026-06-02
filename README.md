<h1 align="center">
  <a href="https://lapce.dev" target="_blank">
  <img src="extra/images/logo.png" width=200 height=200/><br>
  Lapce
  </a>
</h1>

<h4 align="center">Lightning-fast And Powerful Code Editor</h4>

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


Lapce (IPA: /læps/) is written in pure Rust, with a UI in [Floem](https://github.com/lapce/floem). It is designed with [Rope Science](https://xi-editor.io/docs/rope_science_00.html) from the [Xi-Editor](https://github.com/xi-editor/xi-editor), enabling lightning-fast computation, and leverages [wgpu](https://github.com/gfx-rs/wgpu) for rendering. More information about the features of Lapce can be found on the [main website](https://lapce.dev) and user documentation can be found on [GitBook](https://docs.lapce.dev/).

![](https://github.com/lapce/lapce/blob/master/extra/images/screenshot.png?raw=true)

## Features

* Built-in LSP ([Language Server Protocol](https://microsoft.github.io/language-server-protocol/)) support to give you intelligent code features such as: completion, diagnostics and code actions
* Modal editing support as first class citizen (Vim-like, and toggleable)
* Built-in remote development support inspired by [VSCode Remote Development](https://code.visualstudio.com/docs/remote/remote-overview). Enjoy the benefits of a "local" experience, and seamlessly gain the full power of a remote system. We also have [Lapdev](https://lap.dev/) which can help manage your remote dev environments. 
* Plugins can be written in programming languages that can compile to the [WASI](https://wasi.dev/) format (C, Rust, [AssemblyScript](https://www.assemblyscript.org/))
* Built-in terminal, so you can execute commands in your workspace, without leaving Lapce.

## Installation

You can find pre-built releases for Windows, Linux and macOS [here](https://github.com/lapce/lapce/releases), or [installing with a package manager](docs/installing-with-package-manager.md).
If you'd like to compile from source, you can find the [guide](docs/building-from-source.md).

## Contributing

<a href="https://ws.lap.dev/#https://github.com/lapce/lapce" target="_blank">
      <img src="https://lap.dev/images/open-in-lapdev.svg?version=8" alt="Open in Lapdev">
</a>

[Lapdev](https://lap.dev/), developed by the Lapce team, is a cloud dev env service similar to GitHub Codespaces. By clicking the button above, you'll be taken to a fully set up Lapce dev env where you can browse the code and start developing. All dependencies are pre-installed, so you can get straight to code.

Guidelines for contributing to Lapce can be found in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Feedback & Contact

The most popular place for Lapce developers and users is on the [Discord server](https://discord.gg/n8tGJ6Rn6D).

Or, join the discussion on [Reddit](https://www.reddit.com/r/lapce/) where we are just getting started.

There is also a [Matrix Space](https://matrix.to/#/#lapce-editor:matrix.org), which is linked to the content from the Discord server.

## 🏆 SuperInstance Enhancement: Coverage Gap Finder

**87% code coverage. You feel confident.**

But 87% of *what*? Your tests exercise 87% of functions. They cover 23% of execution *paths*.

[Coverage Gap Finder](coverage-gap/) doesn't sell you on confidence. It shows the actual holes.

```bash
cargo llvm-cov --json > coverage.json
cargo run --bin coverage-gap -- coverage.json
```

### How it works

Build a simplicial complex from your test execution traces:

- **Functions = vertices.** Each tested function is a point in code-feature space (branches, loops, match arms, generics, async, unsafe).
- **Tests that call the same function = edges.** If two tests exercise the same function, they're connected.
- **Tests that call 3 functions together = 2-simplices (triangles).** Three functions co-tested form a filled triangle.

Once the complex is built, we compute **Betti numbers** — the homology of your test coverage:

```
β₀ = 8  (8 disconnected test clusters — your tests don't exercise cross-module interactions)
β₁ = 3  (3 holes — function pairs that SHOULD be tested together but aren't)
β₂ = 0  (no 3-way coverage holes — at least your basic interactions work)
```

### That 87%? Look again.

Your 87% coverage is misleading. The 3 holes are at the boundaries between modules — exactly where bugs live.

Here's what `coverage-gap` actually reports on a real Lapce workspace:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Coverage Gap Report
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Lines: 87.3%  |  Functions: 87.0%  |  Branches: 23.1%

 Topological Analysis — Betti Numbers:
   β₀=8 (connected components of tested features),
   β₁=3 (untested feature transitions/holes),
   β₂=0 (voids in feature coverage)

 Features Analyzed: 142 (127 covered, 15 uncovered)
 Gap Score: 16.2

 Top Prioritized Gaps:
   1. 🔴 Critical | src/lsp/handler.rs:203 — unsafe
      🔴 Unsafe block without test coverage — undefined behavior risk
   2. 🟠 High | src/editor/buffer.rs:441 — async
      🟠 Async code uncovered — potential silent failure in error paths
   3. 🟡 Medium | src/plugin/wasi.rs:87 — generics
      🟡 Generic code untested — may have type-level bugs
   4. 🟡 Medium | src/keymap/key.rs:310 — match arms
      🟡 Match arms not fully covered — missed patterns
   5. 🟡 Medium | src/terminal/pty.rs:55 — branches
      🟡 Branch coverage missing — untested code paths
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The number that matters isn't the percentage. It's the Betti numbers. β₁ tells you how many holes are in your test suite — function pairs that interact in production but never in your tests.

3 holes. At module boundaries. That's where bugs breed.

See [`coverage-gap/INTEGRATION.md`](coverage-gap/INTEGRATION.md) for full documentation.

## License

Lapce is released under the Apache License Version 2, which is an open source license. You may contribute to this project, or use the code as you please as long as you adhere to its conditions. You can find a copy of the license text here: [`LICENSE`](LICENSE).
