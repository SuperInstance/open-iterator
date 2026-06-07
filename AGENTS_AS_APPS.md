# Agents as Applications: open-iterator

> The editor doesn't just edit code. It *is* the agent.

## The Paradigm Shift

Traditional code editors are passive tools: they display text, apply syntax highlighting, and wait for the human to type. **open-iterator** breaks that model. The editor is an active participant in the development process. It understands the capability graph of your workspace, suggests integrations across crates, enforces conservation patterns, and ranks code by spectral importance.

The agent *is* the editor. The assistance doesn't happen *to* the developer. The editor *performs* assistance as its native mode of existence.

## How It Works

### CAPABILITY.toml Scanning

Every SuperInstance crate and many third-party libraries now ship with a `CAPABILITY.toml` file — a machine-readable manifest of what the crate can do. open-iterator recursively scans the workspace for these manifests and builds an in-memory capability graph.

```toml
[capability]
name = "my-crate"
version = "0.1.0"
description = "Does important things"

[capability.capabilities]
transform = "fn transform(data: Data) -> Result<Output>"
validate = "fn validate(input: &Input) -> bool"
```

When you open a file, the editor already knows every capability available in your workspace. No documentation lookup needed.

### Import Suggestions

The `suggest_imports()` function matches your current editing context against the capability graph. If you're writing serialization code, the editor knows `serde` is nearby. If you're building a GPU pipeline, it surfaces the rendering crates.

The matching is semantic: it doesn't just grep for names. It scores relevance based on:
- Capability name overlap with your code's vocabulary
- Description keyword matching
- Type signature compatibility hints

### Conservation Law Enforcement (γ + H = C)

SuperInstance's conservation model treats every computational budget as a physical law: **γ (active work) + H (idle/waste) = C (total capacity)**. The editor enforces this in two ways:

1. **Annotation scanning**: Detects `// conservation: γ=X, H=Y, C=Z` or `// SI-CAPACITY: X/Y/Z` comments and flags violations where γ + H > C.
2. **Pattern detection**: Recognizes common overcommit patterns in resource allocation code.

This isn't linting. It's physics. You can't cheat the conservation law any more than you can create energy from nothing.

### Spectral Code Ranking

Files in a codebase aren't equally important. The `spectral_code_ranking()` function constructs a directed import graph, builds a degree-normalized transition matrix, and computes the principal eigenvector via power iteration (PageRank-style).

The result: a ranked list of files by their eigenvalue importance. Hub files (imported by many others) rank highest. Leaf files rank lowest. This tells you:
- Which files to test most thoroughly
- Which refactors have the highest blast radius
- Where to focus documentation efforts

## Architecture

```
lapce-proxy/src/si/
├── mod.rs              — Module declaration
└── agent_assist.rs     — Core agent assistance (all functions + tests)
```

The module lives in `lapce-proxy` because that's where workspace scanning and file system access happen. The proxy already has the infrastructure for watching files and managing workspace state — agent assistance extends that infrastructure with semantic understanding.

## Integration Points

- **open-terminal**: Agent-assisted debugging sessions launched from the editor appear in the terminal overlay
- **open-parallel**: Spectral ranking feeds into parallel task scheduling (important files get priority compilation)
- **open-mind**: Capability discovery enables the induction engine to understand workspace structure
- **open-tui**: Dashboard widgets display conservation budgets and spectral rankings in real-time

## Future Directions

- Real-time conservation budget tracking as you type
- Capability graph visualization in the editor sidebar
- Cross-workspace capability search (find crates across all your projects)
- LSP integration for capability-aware auto-complete
