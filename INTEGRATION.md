# Integration Guide: Ternary Intelligence in Open Iterator (Lapce)

> How the ternary editing tracker, style classifier, and model router plug into Lapce's code editor architecture.

## Overview

Open Iterator (fork of Lapce) integrates ternary signals at the **editing layer** — tracking every code change as a ternary event (+1 added, -1 deleted, 0 unchanged), classifying coding style into strategy species, and routing AI model requests accordingly.

## Ternary Crates & Modules

| Module | File | Role |
|--------|------|------|
| `EditingTracker` | `ternary_integration.rs` | Records per-file edit signals as ternary trits (Added/Idle/Deleted) with sliding window |
| `StyleClassifier` | `ternary_integration.rs` | Classifies editing patterns into 5 strategy species using ternary ratios |
| `ModelRouter` | `ternary_integration.rs` | Routes AI completion requests to the best model based on classified editing style |

## Integration Points

### 1. Editing Tracker → `lapce-core`

The `EditingTracker` hooks into Lapce's text change pipeline to record every edit as a ternary signal:

```rust
// In lapce-core's text change handler
use ternary_integration::{EditingTracker, EditTrit};

let mut tracker = EditingTracker::new(100); // 100-event sliding window

// On text insertion
tracker.record("src/main.rs", EditTrit::Added);

// On text deletion
tracker.record("src/main.rs", EditTrit::Deleted);

// On cursor move without change
tracker.record("src/main.rs", EditTrit::Idle);

// Get editing ratio for a file
let (added, deleted, idle) = tracker.edit_ratio("src/main.rs");
// e.g. (0.6, 0.3, 0.1) — mostly writing, some deletions
```

**Where it connects:** `lapce-core` fires text change events. The tracker subscribes to these events and maintains per-file ternary histories.

### 2. Style Classifier → `lapce-rpc`

The `StyleClassifier` maps editing ratios to one of 5 strategy species, which then informs the editor's behavior:

```rust
use ternary_integration::{StyleClassifier, StrategySpecies};

let classifier = StyleClassifier::new();

// Classify based on accumulated editing patterns
let species = classifier.classify(&tracker, "src/main.rs");

match species {
    StrategySpecies::Constructor  => { /* high Add ratio — suggest completions */ }
    StrategySpecies::Refactorer   => { /* balanced Add/Delete — suggest renames */ }
    StrategySpecies::Debugger     => { /* high Idle + Delete — suggest breakpoints */ }
    StrategySpecies::Explorer     => { /* high Idle — suggest documentation */ }
    StrategySpecies::Integrator   => { /* mixed — suggest test runs */ }
}
```

**Where it connects:** Classification results travel via `lapce-rpc` messages from the core to the UI layer, adjusting the suggestion UI dynamically.

### 3. Model Router → `lapce-proxy`

The `ModelRouter` uses classified editing style to select the optimal AI model for completion requests:

```rust
use ternary_integration::{ModelRouter, StrategySpecies};

let router = ModelRouter::new();

// Route based on editing style
let model = router.route(StrategySpecies::Constructor);
// Returns which model endpoint to use, temperature, max_tokens, etc.

// In lapce-proxy's LSP request handler:
let config = router.route(species);
proxy.send_completion_request(config).await
```

**Where it connects:** `lapce-proxy` handles LSP and AI model communication. The router sits between the proxy's request queue and the model endpoint, adjusting request parameters per editing style.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  lapce-app (UI)                                     │
│  ┌───────────────┐  ┌────────────────────────────┐  │
│  │ Suggestion UI │  │ Status bar (strategy display)│  │
│  └───────┬───────┘  └────────────┬───────────────┘  │
│          │ rpc                     │ rpc              │
│  ┌───────▼────────────────────────▼───────────────┐  │
│  │ lapce-core                                      │  │
│  │  ┌──────────────────┐  ┌──────────────────┐    │  │
│  │  │ EditingTracker   │→ │ StyleClassifier  │    │  │
│  │  │ (edit signals)   │  │ (5 species)      │    │  │
│  │  └──────────────────┘  └────────┬─────────┘    │  │
│  └─────────────────────────────────┼──────────────┘  │
│                                    │                  │
│  ┌─────────────────────────────────▼──────────────┐  │
│  │ lapce-proxy                                     │  │
│  │  ┌──────────────────┐                          │  │
│  │  │ ModelRouter      │ → AI model endpoints     │  │
│  │  │ (style→model)    │                          │  │
│  │  └──────────────────┘                          │  │
│  └────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Committed Files

- `ac85fd0` — `ternary_integration.rs` — full implementation (290 lines): EditingTracker, StyleClassifier with 5 species, ModelRouter

## Adding New Strategy Species

1. Add the species variant to `StrategySpecies` enum in `ternary_integration.rs`
2. Define the classification thresholds in `StyleClassifier::classify()`
3. Map the species to model parameters in `ModelRouter::route()`
4. Update the UI in `lapce-app` to display the new species indicator
