//! Lapce plugin integration for Coverage Gap Finder.
//!
//! This module provides the bridge between the coverage-gap analysis library
//! and the Lapce editor plugin system. It registers commands, panel views,
//! and diagnostic annotations that surface coverage gaps inline.
//!
//! To activate: run `coverage-gap run` from the Lapce command palette or invoke
//! the proxy command from within the editor.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parse::{self, CoverageData};
use crate::report::{CoverageGapReport, Priority, build_gap_entries};
use crate::simplicial::{self, CoverageFeature};

/// Result of running coverage gap analysis in a workspace.
#[derive(Debug, Clone)]
pub struct CoverageGapPluginResult {
    /// The report text.
    pub report: CoverageGapReport,
    /// Per-file gap annotations for inline display.
    pub file_gaps: HashMap<PathBuf, Vec<InlineGap>>,
}

/// An inline gap annotation shown in the editor.
#[derive(Debug, Clone)]
pub struct InlineGap {
    pub line: usize,
    pub message: String,
    pub severity: InlineSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineSeverity {
    Error,
    Warning,
    Hint,
}

/// Run coverage gap analysis given a path to a coverage JSON file and
/// the workspace root.
pub fn run_coverage_gap(
    coverage_path: &Path,
    workspace_root: &Path,
) -> anyhow::Result<CoverageGapPluginResult> {
    let json = std::fs::read_to_string(coverage_path)?;
    let data: CoverageData = parse::parse_coverage_json(&json)?;

    // Extract totals from first file data
    let totals = data.data.first()
        .map(|d| d.totals.clone())
        .unwrap_or_default();

    // Extract function coverage
    let functions = parse::extract_function_coverage(&data);

    // Build features from coverage data
    let features = simplicial::extract_features_from_coverage(&functions);

    // Build simplicial complex and compute Betti numbers
    let threshold = compute_adaptive_threshold(&features);
    let (_simplices, betti) = simplicial::build_feature_complex(&features, threshold);

    // Build gap entries
    let gaps = build_gap_entries(&features, &betti);

    // Build report
    let report = CoverageGapReport::new(&totals, betti, &features, gaps);

    // Build per-file inline annotations
    let file_gaps = build_file_gaps(&features, &report, workspace_root);

    Ok(CoverageGapPluginResult { report, file_gaps })
}

/// Compute an adaptive threshold based on feature density.
fn compute_adaptive_threshold(features: &[CoverageFeature]) -> f64 {
    if features.len() < 2 {
        return 1.0;
    }

    let mut distances = Vec::new();
    for i in 0..features.len().min(50) {
        for j in (i + 1)..features.len().min(50) {
            let d = simplicial::euclidean_sq(&features[i].vector, &features[j].vector);
            distances.push(d);
        }
    }

    if distances.is_empty() {
        return 1.0;
    }

    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = distances[distances.len() / 2];
    (median.sqrt().max(0.5)).min(3.0)
}

/// Build per-file inline annotations from the report.
fn build_file_gaps(
    _features: &[CoverageFeature],
    report: &CoverageGapReport,
    workspace_root: &Path,
) -> HashMap<PathBuf, Vec<InlineGap>> {
    let mut file_gaps: HashMap<PathBuf, Vec<InlineGap>> = HashMap::new();

    for entry in &report.gaps {
        // Parse location string "file.rs:line" or "gap:file.rs:..."
        let (file_str, line_str) = if let Some(idx) = entry.location.rfind(':') {
            let (f, l) = entry.location.split_at(idx);
            (f, &l[1..])
        } else {
            (&entry.location[..], "0")
        };

        // Clean up "gap:" prefix if present
        let file_str = file_str.strip_prefix("gap:").unwrap_or(file_str);

        let line: usize = line_str.parse().unwrap_or(0);
        let path = PathBuf::from(file_str);

        let (severity, message) = match entry.priority {
            Priority::Critical => (
                InlineSeverity::Error,
                format!("[Coverage Gap] {} — {}", entry.feature_type, entry.reason),
            ),
            Priority::High => (
                InlineSeverity::Warning,
                format!("[Coverage Gap] {} — {}", entry.feature_type, entry.reason),
            ),
            _ => (
                InlineSeverity::Hint,
                format!("[Coverage Gap] {} — {}", entry.feature_type, entry.reason),
            ),
        };

        // Resolve relative to workspace root if needed
        let resolved = if path.is_relative() {
            workspace_root.join(&path)
        } else {
            path
        };

        file_gaps
            .entry(resolved)
            .or_default()
            .push(InlineGap { line, message, severity });
    }

    file_gaps
}

/// Generate a plugin manifest for Lapce plugin system.
///
/// Outputs a `lapce-plugin.toml`-compatible metadata struct.
pub fn plugin_manifest() -> serde_json::Value {
    serde_json::json!({
        "name": "coverage-gap",
        "version": "0.4.6",
        "author": "SuperInstance",
        "description": "Topological coverage gap finder — Betti numbers for test blind spots",
        "display_name": "Coverage Gap Finder",
        "repository": "https://github.com/SuperInstance/lapce",
        "wasm": false,
        "activation": {
            "workspace_contains": ["coverage.json", "lcov.info", "target/coverage"]
        },
        "config": {
            "coverage_gap.threshold": {
                "default": "auto",
                "description": "Simplicial complex distance threshold (auto|0.5|1.0|2.0|3.0)"
            },
            "coverage_gap.max_gaps": {
                "default": 20,
                "description": "Maximum number of gaps to display at once"
            }
        }
    })
}

/// Determine whether a workspace looks like it has coverage data.
pub fn has_coverage_data(workspace_root: &Path) -> bool {
    let candidates = [
        "coverage.json",
        "target/coverage/coverage.json",
        "lcov.info",
        "target/coverage/lcov.info",
        "target/llvm-cov/coverage.json",
    ];

    candidates.iter().any(|c| workspace_root.join(c).exists())
}

/// Find the most recent coverage JSON file in a workspace.
pub fn find_coverage_json(workspace_root: &Path) -> Option<PathBuf> {
    let candidates = [
        "target/llvm-cov/coverage.json",
        "target/coverage/coverage.json",
        "coverage.json",
    ];

    for candidate in &candidates {
        let path = workspace_root.join(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest() {
        let manifest = plugin_manifest();
        assert_eq!(manifest["name"], "coverage-gap");
        assert_eq!(manifest["author"], "SuperInstance");
        assert!(manifest["config"]["coverage_gap.threshold"].is_object());
    }

    #[test]
    fn test_has_coverage_data() {
        let dir = std::env::temp_dir().join("cov-test");
        let _ = std::fs::create_dir_all(&dir);
        // Should return false for empty dir
        assert!(!has_coverage_data(&dir));
        std::fs::write(dir.join("coverage.json"), "{}").ok();
        assert!(has_coverage_data(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_coverage_json() {
        let dir = std::env::temp_dir().join("cov-find-test");
        let _ = std::fs::create_dir_all(&dir);
        assert!(find_coverage_json(&dir).is_none());

        let nested = dir.join("target").join("llvm-cov");
        let _ = std::fs::create_dir_all(&nested);
        std::fs::write(nested.join("coverage.json"), "{}").ok();
        assert!(find_coverage_json(&dir).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
