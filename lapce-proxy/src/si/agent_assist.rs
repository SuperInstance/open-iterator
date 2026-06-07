//! Agent-assisted coding for open-iterator.
//!
//! Reads CAPABILITY.toml files from the workspace to understand available
//! capabilities and provides three core functions:
//!
//! - [`suggest_imports`] — scans workspace for CAPABILITY.toml, suggests relevant crate imports
//! - [`detect_conservation_violation`] — checks if code respects γ+H=C budget patterns
//! - [`spectral_code_ranking`] — ranks files by importance using eigenvalue decomposition

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Parsed representation of a CAPABILITY.toml file.
#[derive(Debug, Clone)]
pub struct CapabilityManifest {
    /// Name of the crate or module.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// Exported capabilities (name → type signature or brief description).
    pub capabilities: HashMap<String, String>,
    /// Dependencies on other capability-bearing crates.
    pub depends: Vec<String>,
    /// Path where this manifest was found.
    pub source_path: PathBuf,
}

/// An import suggestion with relevance context.
#[derive(Debug, Clone)]
pub struct ImportSuggestion {
    /// The crate or module path to import.
    pub import_path: String,
    /// Why this import is suggested.
    pub reason: String,
    /// Relevance score 0.0–1.0.
    pub relevance: f64,
    /// Source CAPABILITY.toml.
    pub from_manifest: String,
}

/// Conservation law parameters: γ (active work) + H (idle/waste) = C (total budget).
#[derive(Debug, Clone, Copy)]
pub struct ConservationBudget {
    /// Active work allocation (γ).
    pub gamma: f64,
    /// Idle/waste allocation (H).
    pub eta: f64,
    /// Total budget (C).
    pub capacity: f64,
}

impl ConservationBudget {
    /// Check whether γ + H ≤ C (within tolerance).
    pub fn is_conserved(&self, tolerance: f64) -> bool {
        let sum = self.gamma + self.eta;
        sum <= self.capacity + tolerance
    }

    /// Compute the violation magnitude: (γ + H) - C.
    /// Positive means overcommit; zero or negative means conserved.
    pub fn violation(&self) -> f64 {
        (self.gamma + self.eta) - self.capacity
    }
}

/// Result of conservation violation detection on a piece of code.
#[derive(Debug, Clone)]
pub struct ConservationViolation {
    /// File path where the violation was found.
    pub file: String,
    /// Line number (1-indexed), if applicable.
    pub line: Option<usize>,
    /// The detected budget parameters.
    pub budget: ConservationBudget,
    /// Human-readable description of the violation.
    pub message: String,
}

/// A file ranked by spectral importance.
#[derive(Debug, Clone)]
pub struct RankedFile {
    /// File path.
    pub path: PathBuf,
    /// Eigenvalue-based importance score (principal eigenvalue contribution).
    pub importance: f64,
    /// Number of inbound imports (how many files depend on this one).
    pub inbound_degree: usize,
    /// Number of outbound imports (how many files this one depends on).
    pub outbound_degree: usize,
}

/// Parse a single CAPABILITY.toml from disk.
///
/// Accepts a minimal TOML structure:
/// ```toml
/// [capability]
/// name = "my-crate"
/// version = "0.1.0"
/// description = "Does things"
///
/// [capability.capabilities]
/// run = "fn run(ctx: Context) -> Result"
/// compute = "fn compute(input: Input) -> Output"
///
/// [capability.depends]
/// depends = ["other-crate"]
/// ```
pub fn parse_capability_toml(path: &Path) -> Result<CapabilityManifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    // Minimal TOML parser — extracts [capability] table without full TOML crate.
    let mut name = String::new();
    let mut version = String::new();
    let mut description = String::new();
    let mut capabilities = HashMap::new();
    let mut depends = Vec::new();

    let mut in_capability = false;
    let mut in_capabilities = false;
    let mut in_depends_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[capability]" {
            in_capability = true;
            in_capabilities = false;
            in_depends_section = false;
            continue;
        }
        if trimmed == "[capability.capabilities]" {
            in_capability = false;
            in_capabilities = true;
            in_depends_section = false;
            continue;
        }
        if trimmed.starts_with("[capability.depends]") || trimmed == "[capability.depends]" {
            in_capability = false;
            in_capabilities = false;
            in_depends_section = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_capability = false;
            in_capabilities = false;
            in_depends_section = false;
            continue;
        }

        if in_capability {
            if let Some((key, val)) = parse_kv(trimmed) {
                match key {
                    "name" => name = unquote(&val),
                    "version" => version = unquote(&val),
                    "description" => description = unquote(&val),
                    _ => {}
                }
            }
        } else if in_capabilities {
            if let Some((key, val)) = parse_kv(trimmed) {
                capabilities.insert(key.to_string(), unquote(&val));
            }
        } else if in_depends_section {
            // Parse depends = ["a", "b"] or per-line items
            if let Some((key, val)) = parse_kv(trimmed) {
                if key == "depends" {
                    depends = parse_string_array(&val);
                }
            }
        }
    }

    Ok(CapabilityManifest {
        name,
        version,
        description,
        capabilities,
        depends,
        source_path: path.to_path_buf(),
    })
}

/// Scan a workspace directory recursively for all CAPABILITY.toml files.
pub fn scan_workspace_capabilities(workspace_root: &Path) -> Vec<CapabilityManifest> {
    let mut manifests = Vec::new();
    if let Ok(entries) = walk_dir_recursive(workspace_root) {
        for path in entries {
            if path.file_name().map(|n| n == "CAPABILITY.toml").unwrap_or(false) {
                if let Ok(manifest) = parse_capability_toml(&path) {
                    manifests.push(manifest);
                }
            }
        }
    }
    manifests
}

/// Suggest imports based on the current editing context.
///
/// Scans the workspace for CAPABILITY.toml files and matches the provided
/// context string (typically the current file's content or a selection)
/// against the capabilities described in each manifest.
pub fn suggest_imports(context: &str, manifests: &[CapabilityManifest]) -> Vec<ImportSuggestion> {
    let context_lower = context.to_lowercase();
    let mut suggestions = Vec::new();

    // Build keyword sets from context
    let context_keywords: Vec<&str> = context_lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| s.len() > 2)
        .collect();

    for manifest in manifests {
        let manifest_lower = manifest.name.to_lowercase();
        let desc_lower = manifest.description.to_lowercase();

        // Score based on keyword overlap with manifest name and description
        let mut score: f64 = 0.0;

        for kw in &context_keywords {
            if manifest_lower.contains(kw) {
                score += 0.3;
            }
            if desc_lower.contains(kw) {
                score += 0.2;
            }
            for (cap_name, cap_sig) in &manifest.capabilities {
                let cap_lower = cap_name.to_lowercase();
                let sig_lower = cap_sig.to_lowercase();
                if cap_lower.contains(kw) || sig_lower.contains(kw) {
                    score += 0.15;
                }
            }
        }

        // Cap score at 1.0
        let score = score.min(1.0);

        if score > 0.1 {
            suggestions.push(ImportSuggestion {
                import_path: manifest.name.clone(),
                reason: format!(
                    "Matches context keywords. {}",
                    manifest.description
                ),
                relevance: score,
                from_manifest: manifest.name.clone(),
            });
        }
    }

    // Sort by relevance descending
    suggestions.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));
    suggestions
}

/// Detect conservation law violations in source code.
///
/// Scans for comments or annotations of the form:
/// - `// conservation: γ=X, H=Y, C=Z`
/// - `// budget: gamma=X eta=Y capacity=Z`
/// - `// SI-CAPACITY: X/Y/Z`
///
/// Returns any violations where γ + H > C (beyond tolerance).
pub fn detect_conservation_violation(code: &str, file_path: &str, tolerance: f64) -> Vec<ConservationViolation> {
    let mut violations = Vec::new();

    for (line_idx, line) in code.lines().enumerate() {
        let trimmed = line.trim();

        // Pattern 1: // conservation: γ=X, H=Y, C=Z
        if let Some(budget) = parse_conservation_comment(trimmed) {
            if !budget.is_conserved(tolerance) {
                violations.push(ConservationViolation {
                    file: file_path.to_string(),
                    line: Some(line_idx + 1),
                    budget,
                    message: format!(
                        "Conservation violation: γ({}) + H({}) = {} > C({}) by {}",
                        budget.gamma,
                        budget.eta,
                        budget.gamma + budget.eta,
                        budget.capacity,
                        budget.violation()
                    ),
                });
            }
        }

        // Pattern 2: // SI-CAPACITY: X/Y/Z  (gamma/eta/capacity)
        if let Some(rest) = trimmed.strip_prefix("// SI-CAPACITY:") {
            let parts: Vec<&str> = rest.trim().split('/').collect();
            if parts.len() == 3 {
                if let (Ok(g), Ok(h), Ok(c)) = (
                    parts[0].trim().parse::<f64>(),
                    parts[1].trim().parse::<f64>(),
                    parts[2].trim().parse::<f64>(),
                ) {
                    let budget = ConservationBudget {
                        gamma: g,
                        eta: h,
                        capacity: c,
                    };
                    let violation = budget.violation();
                    if !budget.is_conserved(tolerance) {
                        violations.push(ConservationViolation {
                            file: file_path.to_string(),
                            line: Some(line_idx + 1),
                            budget,
                            message: format!(
                                "SI-CAPACITY violation: {g}/{h}/{c} overcommitted by {violation}"
                            ),
                        });
                    }
                }
            }
        }
    }

    violations
}

/// Rank files by spectral importance using eigenvalue decomposition of the import graph.
///
/// Constructs an adjacency matrix from the import graph (files × files),
/// computes the degree-normalized transition matrix, and uses power iteration
/// to approximate the principal eigenvector (PageRank-style).
pub fn spectral_code_ranking(files: &[(PathBuf, Vec<String>)]) -> Vec<RankedFile> {
    let n = files.len();
    if n == 0 {
        return Vec::new();
    }

    // Build file name → index mapping
    let file_names: Vec<String> = files
        .iter()
        .map(|(p)| {
            p.0.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        })
        .collect();

    let name_to_idx: HashMap<&str, usize> = file_names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), i))
        .collect();

    // Build adjacency matrix (directed: file[i] imports file[j])
    let mut adj: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    let mut outbound: Vec<usize> = vec![0; n];
    let mut inbound: Vec<usize> = vec![0; n];

    for (i, (_, imports)) in files.iter().enumerate() {
        for imp in imports {
            // Try to match import to a file
            if let Some(&j) = name_to_idx.get(imp.as_str()) {
                if i != j {
                    adj[i][j] = 1.0;
                    outbound[i] += 1;
                    inbound[j] += 1;
                }
            }
        }
    }

    // Normalize rows to create transition matrix (add damping for dangling nodes)
    let damping = 0.15;
    let teleport = 1.0 / n as f64;

    let mut transition: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    for i in 0..n {
        let row_sum: f64 = adj[i].iter().sum();
        for j in 0..n {
            if row_sum > 0.0 {
                transition[i][j] = (1.0 - damping) * (adj[i][j] / row_sum) + damping * teleport;
            } else {
                transition[i][j] = teleport;
            }
        }
    }

    // Power iteration to find principal eigenvector
    let mut rank: Vec<f64> = vec![1.0 / n as f64; n];
    let iterations = 50;

    for _ in 0..iterations {
        let mut new_rank = vec![0.0; n];
        for j in 0..n {
            for i in 0..n {
                new_rank[j] += transition[i][j] * rank[i];
            }
        }
        // Normalize
        let sum: f64 = new_rank.iter().sum();
        if sum > 0.0 {
            for r in new_rank.iter_mut() {
                *r /= sum;
            }
        }
        rank = new_rank;
    }

    // Scale to [0, 1] range
    let max_rank = rank.iter().cloned().fold(0.0_f64, f64::max);
    if max_rank > 0.0 {
        for r in rank.iter_mut() {
            *r /= max_rank;
        }
    }

    let mut results: Vec<RankedFile> = files
        .iter()
        .enumerate()
        .map(|(i, (path, _))| RankedFile {
            path: path.clone(),
            importance: rank[i],
            inbound_degree: inbound[i],
            outbound_degree: outbound[i],
        })
        .collect();

    results.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
    results
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn parse_kv(line: &str) -> Option<(&str, &str)> {
    let eq = line.find('=')?;
    Some((line[..eq].trim(), &line[eq + 1..].trim()))
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_string_array(s: &str) -> Vec<String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return Vec::new();
    }
    let inner = &s[1..s.len() - 1];
    inner
        .split(',')
        .map(|item| unquote(item.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}



fn extract_param_value(part: &str, keys: &[&str]) -> Option<f64> {
    let lower = part.to_lowercase();
    for key in keys {
        if let Some(rest) = lower.strip_prefix(key) {
            let rest = rest.trim_start_matches(|c: char| c == '=' || c == ':' || c == ' ');
            if !rest.is_empty() {
                return rest.parse().ok();
            }
        }
    }
    None
}

fn parse_conservation_comment(line: &str) -> Option<ConservationBudget> {
    let lower = line.to_lowercase();

    if !lower.contains("conservation:") && !lower.contains("budget:") {
        return None;
    }

    let mut gamma = None;
    let mut eta = None;
    let mut cap = None;

    for part in line.split(&[',', ' ', ';']) {
        let part = part.trim();
        if gamma.is_none() {
            gamma = extract_param_value(part, &["γ", "gamma"]);
        }
        if eta.is_none() {
            eta = extract_param_value(part, &["η", "h", "eta"]);
        }
        if cap.is_none() {
            cap = extract_param_value(part, &["c", "capacity"]);
        }
    }

    match (gamma, eta, cap) {
        (Some(g), Some(h), Some(c)) => Some(ConservationBudget {
            gamma: g,
            eta: h,
            capacity: c,
        }),
        _ => None,
    }
}

fn walk_dir_recursive(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut result = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip hidden dirs and target
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if !name.starts_with('.') && name != "target" && name != "node_modules" {
                        stack.push(path);
                    }
                } else {
                    result.push(path);
                }
            }
        }
    }

    Ok(result)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // --- parse_capability_toml ---

    #[test]
    fn test_parse_basic_capability_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CAPABILITY.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"
[capability]
name = "test-crate"
version = "0.1.0"
description = "A test crate"

[capability.capabilities]
run = "fn run() -> Result"
compute = "fn compute(x: i32) -> i32"

[capability.depends]
depends = ["other-crate", "serde"]
"#
        )
        .unwrap();

        let manifest = parse_capability_toml(&path).unwrap();
        assert_eq!(manifest.name, "test-crate");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.description, "A test crate");
        assert_eq!(manifest.capabilities.len(), 2);
        assert!(manifest.capabilities.contains_key("run"));
        assert_eq!(manifest.depends.len(), 2);
        assert!(manifest.depends.contains(&"other-crate".to_string()));
    }

    #[test]
    fn test_parse_capability_missing_file() {
        let result = parse_capability_toml(Path::new("/nonexistent/CAPABILITY.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_capability_minimal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CAPABILITY.toml");
        std::fs::write(&path, "[capability]\nname = \"minimal\"\nversion = \"0.0.1\"\n").unwrap();
        let manifest = parse_capability_toml(&path).unwrap();
        assert_eq!(manifest.name, "minimal");
        assert!(manifest.capabilities.is_empty());
        assert!(manifest.depends.is_empty());
    }

    // --- suggest_imports ---

    #[test]
    fn test_suggest_imports_basic() {
        let manifests = vec![CapabilityManifest {
            name: "serde".to_string(),
            version: "1.0".to_string(),
            description: "Serialization framework".to_string(),
            capabilities: {
                let mut m = HashMap::new();
                m.insert("serialize".to_string(), "fn serialize<T>(val: &T)".to_string());
                m
            },
            depends: vec![],
            source_path: PathBuf::from("/fake/CAPABILITY.toml"),
        }];

        let suggestions = suggest_imports("I need to serialize my data", &manifests);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].import_path, "serde");
        assert!(suggestions[0].relevance > 0.0);
    }

    #[test]
    fn test_suggest_imports_no_match() {
        let manifests = vec![CapabilityManifest {
            name: "gpu-renderer".to_string(),
            version: "0.1".to_string(),
            description: "GPU rendering engine".to_string(),
            capabilities: HashMap::new(),
            depends: vec![],
            source_path: PathBuf::from("/fake/CAPABILITY.toml"),
        }];

        let suggestions = suggest_imports("I want to parse CSV files", &manifests);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_suggest_imports_ranking() {
        let manifests = vec![
            CapabilityManifest {
                name: "csv-parser".to_string(),
                version: "0.1".to_string(),
                description: "Parse CSV files efficiently".to_string(),
                capabilities: HashMap::new(),
                depends: vec![],
                source_path: PathBuf::from("/fake/a/CAPABILITY.toml"),
            },
            CapabilityManifest {
                name: "csv-writer".to_string(),
                version: "0.1".to_string(),
                description: "Write CSV output".to_string(),
                capabilities: HashMap::new(),
                depends: vec![],
                source_path: PathBuf::from("/fake/b/CAPABILITY.toml"),
            },
        ];

        let suggestions = suggest_imports("parse CSV files", &manifests);
        assert!(suggestions.len() >= 1);
        // csv-parser should rank higher for "parse CSV files"
        assert_eq!(suggestions[0].import_path, "csv-parser");
    }

    // --- conservation ---

    #[test]
    fn test_conservation_budget_conserved() {
        let budget = ConservationBudget {
            gamma: 0.4,
            eta: 0.5,
            capacity: 1.0,
        };
        assert!(budget.is_conserved(0.01));
        assert!(budget.violation() <= 0.0);
    }

    #[test]
    fn test_conservation_budget_violated() {
        let budget = ConservationBudget {
            gamma: 0.7,
            eta: 0.5,
            capacity: 1.0,
        };
        assert!(!budget.is_conserved(0.01));
        assert!(budget.violation() > 0.0);
        assert!((budget.violation() - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_detect_conservation_violation_clean() {
        let code = r#"
fn main() {
    // conservation: gamma=0.3, eta=0.5, capacity=1.0
    println!("hello");
}
"#;
        let violations = detect_conservation_violation(code, "main.rs", 0.01);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_detect_conservation_violation_flagged() {
        let code = r#"
fn main() {
    // conservation: γ=0.8, H=0.4, C=1.0
    println!("overcommitted!");
}
"#;
        let violations = detect_conservation_violation(code, "main.rs", 0.01);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, Some(3));
        assert!(violations[0].message.contains("violation"));
    }

    #[test]
    fn test_detect_si_capacity_violation() {
        let code = "// SI-CAPACITY: 0.8/0.4/1.0\nfn over() {}";
        let violations = detect_conservation_violation(code, "test.rs", 0.01);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("SI-CAPACITY"));
    }

    #[test]
    fn test_detect_conservation_no_annotations() {
        let code = "fn main() { println!(\"no annotations\"); }";
        let violations = detect_conservation_violation(code, "clean.rs", 0.01);
        assert!(violations.is_empty());
    }

    // --- spectral_code_ranking ---

    #[test]
    fn test_spectral_ranking_empty() {
        let files: Vec<(PathBuf, Vec<String>)> = vec![];
        let ranking = spectral_code_ranking(&files);
        assert!(ranking.is_empty());
    }

    #[test]
    fn test_spectral_ranking_single_file() {
        let files = vec![(PathBuf::from("main.rs"), vec![])];
        let ranking = spectral_code_ranking(&files);
        assert_eq!(ranking.len(), 1);
        assert_eq!(ranking[0].path, PathBuf::from("main.rs"));
    }

    #[test]
    fn test_spectral_ranking_hub_file() {
        // utils should rank highest as both main and lib import it
        let files = vec![
            (PathBuf::from("main.rs"), vec!["utils".to_string(), "lib".to_string()]),
            (PathBuf::from("lib.rs"), vec!["utils".to_string()]),
            (PathBuf::from("utils.rs"), vec![]),
        ];
        let ranking = spectral_code_ranking(&files);
        assert_eq!(ranking.len(), 3);
        // utils.rs should have highest importance (most inbound)
        assert_eq!(ranking[0].path, PathBuf::from("utils.rs"));
        assert!(ranking[0].importance > 0.0);
    }

    #[test]
    fn test_spectral_ranking_inbound_outbound_counts() {
        let files = vec![
            (PathBuf::from("a.rs"), vec!["b".to_string()]),
            (PathBuf::from("b.rs"), vec![]),
        ];
        let ranking = spectral_code_ranking(&files);
        // b has 1 inbound, a has 0 inbound
        let b = ranking.iter().find(|r| r.path == PathBuf::from("b.rs")).unwrap();
        let a = ranking.iter().find(|r| r.path == PathBuf::from("a.rs")).unwrap();
        assert_eq!(b.inbound_degree, 1);
        assert_eq!(a.outbound_degree, 1);
    }
}
