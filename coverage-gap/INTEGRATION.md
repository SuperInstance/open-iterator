# SuperInstance Coverage Gap Finder — Integration Guide

## Overview

The Coverage Gap Finder brings **topological data analysis** to Rust test coverage.
It treats your codebase as a simplicial complex — where each code feature (branches,
loops, match arms, generics, async, unsafe) is a point in feature-space, and coverage
holes are topological defects measured by **Betti numbers**.

**β₁ > 0** means your tests missed an entire *family* of code paths — not just lines.

## Architecture

```
┌──────────────────────┐     ┌──────────────────────┐     ┌──────────────────────┐
│  rustc/llvm-cov      │────▶│  coverage-gap CLI     │────▶│  CoverageGapReport   │
│  coverage.json       │     │  (coverage-gap/src/)  │     │  - Betti numbers     │
└──────────────────────┘     │                      │     │  - Priority gaps     │
                             │ 1. Parse JSON         │     │  - Inline annots     │
┌──────────────────────┐     │ 2. Extract features   │     └──────────────────────┘
│  Lapce Plugin         │◀───│ 3. Build complex      │              │
│  (coverage-gap::plugin)│   │ 4. Compute Betti       │              ▼
│  - Inline diagnostics │    │ 5. Rank gaps          │     ┌──────────────────────┐
│  - Command palette    │    └──────────────────────┘     │  lapce-app editor     │
│  - Panel view         │                                  │  inline annotations   │
└──────────────────────┘                                  └──────────────────────┘
```

## Modules

| Module | Description |
|--------|-------------|
| `parse` | Parse `llvm-cov export` JSON into typed coverage data |
| `simplicial` | Build Vietoris-Rips complex, compute Betti β₀, β₁, β₂ |
| `report` | Create prioritized `CoverageGapReport` |
| `plugin` | Lapce editor integration bridge |

## Usage

### CLI

```bash
# Generate coverage data
cargo llvm-cov --json > target/llvm-cov/coverage.json

# Run the coverage gap finder
cargo run --bin coverage-gap -- target/llvm-cov/coverage.json

# Auto-detect in current project
cargo run --bin coverage-gap -- --project .

# JSON output for programmatic consumption
cargo run --bin coverage-gap -- -j
```

### In Lapce

1. Open a Rust project in Lapce
2. Run `cargo llvm-cov --json` in the terminal
3. Run `coverage-gap` from the command palette
4. Gaps appear as inline diagnostics in the editor

### Via proxy integration

The `coverage-gap::plugin` module provides:

- `run_coverage_gap()` — full analysis pipeline
- `has_coverage_data()` — workspace detection
- `find_coverage_json()` — auto-locate coverage output
- `plugin_manifest()` — Lapce plugin metadata

## Betti Numbers Explained

| Betti | Meaning | What to look for |
|-------|---------|-----------------|
| β₀ | Connected components of tested features | Fragmented test coverage |
| β₁ | Holes — untested feature transitions | Missing error handling, branch families |
| β₂ | Voids — missing higher-dim feature combos | E.g., async+unsafe+generic combos untested |

**Example:** β₁=3 means there are 3 independent "holes" in your test coverage —
entire feature classes that tests don't reach.

## Priority Ranking

| Priority | Criteria |
|----------|----------|
| 🔴 Critical | Unsafe code without tests (UB risk) |
| 🟠 High | Async, generic-heavy code untested |
| 🟡 Medium | Branch/loop/match coverage gaps |
| 🟢 Low | Minor or edge-case gaps |

## Configuration

```toml
# In Lapce settings
coverage_gap.threshold = "auto"    # simplicial distance threshold
coverage_gap.max_gaps = 20         # max gaps to display
```

## Dependencies

- `serde` / `serde_json` — coverage JSON parsing
- `ordered-float` — feature-space distance computations
- `rayon` — parallel feature extraction (optional)

## Related

- [README.md](../README.md) — project overview
- [Coverage Gap Finder source](src/) — full implementation
- [Lapce plugin system](../lapce-proxy/src/plugin/) — plugin architecture
