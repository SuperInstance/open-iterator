//! Parse rustc/llvm-cov JSON export format into structured coverage data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level coverage data from `llvm-cov export` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageData {
    pub data: Vec<FileCoverage>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub type_: String,
}

/// Per-file coverage data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    #[serde(default)]
    pub files: Vec<SourceFile>,
    #[serde(default)]
    pub totals: Totals,
}

/// A single source file with coverage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub filename: String,
    #[serde(default)]
    pub segments: Vec<Segment>,
    #[serde(default)]
    pub expansions: Vec<Expansion>,
    #[serde(default)]
    pub summary: Option<FileSummary>,
}

/// A line segment with execution count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub line: usize,
    pub col: usize,
    pub count: u64,
    #[serde(default)]
    pub has_count: bool,
    #[serde(default)]
    pub is_region_entry: bool,
    #[serde(default)]
    pub is_gap_region: bool,
}

/// Expansion regions (macros, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expansion {
    pub file_region: SourceRegion,
    pub target_region: Box<SourceRegion>,
    pub expansions: Vec<Expansion>,
}

/// A source region (line/col range).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRegion {
    pub line_start: usize,
    pub col_start: usize,
    pub line_end: usize,
    pub col_end: usize,
}

/// Summary totals for a file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileSummary {
    pub lines: CountSummary,
    pub regions: CountSummary,
    pub functions: CountSummary,
    pub branches: CountSummary,
    pub instantiations: Option<CountSummary>,
}

/// Count/percent summary.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CountSummary {
    pub count: u64,
    pub covered: u64,
    pub percent: f64,
}

/// Totals across all files.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Totals {
    pub lines: CountSummary,
    pub regions: CountSummary,
    pub functions: CountSummary,
    pub branches: CountSummary,
}

/// Parsed per-function coverage data (inferred from segments).
#[derive(Debug, Clone)]
pub struct FunctionCoverage {
    pub name: String,
    pub file: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub executed: bool,
    pub hit_count: u64,
}

/// A contiguous region of coverage data.
#[derive(Debug, Clone)]
pub struct RegionCoverage {
    pub file: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub count: u64,
    pub is_covered: bool,
}

/// Parse coverage JSON from a string.
pub fn parse_coverage_json(json: &str) -> anyhow::Result<CoverageData> {
    let data: CoverageData = serde_json::from_str(json)?;
    Ok(data)
}

/// Extract function-level coverage from the raw segments.
/// This does a simple heuristic: group segments by runs of zero/non-zero count.
pub fn extract_function_coverage(data: &CoverageData) -> Vec<FunctionCoverage> {
    let mut functions = Vec::new();

    for file_data in &data.data {
        for source in &file_data.files {
            let path = PathBuf::from(&source.filename);
            let mut segments_by_line: HashMap<usize, Vec<&Segment>> = HashMap::new();
            for seg in &source.segments {
                segments_by_line.entry(seg.line).or_default().push(seg);
            }

            // Detect function-like boundaries: where coverage count transitions
            // or where is_region_entry is set.
            let mut in_function = false;
            let mut fn_start = 0usize;
            let mut fn_hit: u64 = 0;
            let mut lines = segments_by_line.keys().copied().collect::<Vec<_>>();
            lines.sort();

            for &line in &lines {
                let segs = &segments_by_line[&line];
                let max_count = segs.iter().map(|s| s.count).max().unwrap_or(0);
                let has_entry = segs.iter().any(|s| s.is_region_entry);

                if !in_function && has_entry {
                    in_function = true;
                    fn_start = line;
                    fn_hit = max_count;
                } else if in_function
                    && (has_entry || max_count == 0)
                    && line > fn_start + 1
                {
                    functions.push(FunctionCoverage {
                        name: format!("fn_at_line_{}", fn_start),
                        file: path.clone(),
                        line_start: fn_start,
                        line_end: line - 1,
                        executed: fn_hit > 0,
                        hit_count: fn_hit,
                    });
                    if has_entry {
                        fn_start = line;
                        fn_hit = max_count;
                    } else {
                        in_function = false;
                    }
                } else if in_function {
                    fn_hit = fn_hit.max(max_count);
                }
            }
        }
    }

    functions
}

/// Extract region-level coverage data.
pub fn extract_regions(data: &CoverageData) -> Vec<RegionCoverage> {
    let mut regions = Vec::new();

    for file_data in &data.data {
        for source in &file_data.files {
            let path = PathBuf::from(&source.filename);
            for seg in &source.segments {
                if seg.is_region_entry {
                    regions.push(RegionCoverage {
                        file: path.clone(),
                        line_start: seg.line,
                        line_end: seg.line,
                        count: seg.count,
                        is_covered: seg.count > 0,
                    });
                }
            }
        }
    }

    regions
}

impl Totals {
    /// Overall line coverage percentage.
    pub fn line_percent(&self) -> f64 {
        self.lines.percent
    }

    /// Overall function coverage percentage.
    pub fn function_percent(&self) -> f64 {
        self.functions.percent
    }

    /// Overall branch coverage percentage.
    pub fn branch_percent(&self) -> f64 {
        self.branches.percent
    }

    /// Overall region coverage percentage.
    pub fn region_percent(&self) -> f64 {
        self.regions.percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_coverage() {
        let json = r#"{
            "data": [{
                "files": [{
                    "filename": "src/main.rs",
                    "segments": [
                        {"line": 1, "col": 1, "count": 1, "has_count": true, "is_region_entry": true, "is_gap_region": false},
                        {"line": 2, "col": 1, "count": 5, "has_count": true, "is_region_entry": true, "is_gap_region": false},
                        {"line": 3, "col": 1, "count": 0, "has_count": true, "is_region_entry": true, "is_gap_region": true}
                    ]
                }],
                "totals": {
                    "lines": {"count": 3, "covered": 2, "percent": 66.67},
                    "regions": {"count": 3, "covered": 2, "percent": 66.67},
                    "functions": {"count": 2, "covered": 1, "percent": 50.0},
                    "branches": {"count": 0, "covered": 0, "percent": 100.0}
                }
            }],
            "version": "2",
            "type_": "llvm.coverage.json.export"
        }"#;

        let data = parse_coverage_json(json).unwrap();
        assert_eq!(data.data.len(), 1);
        assert_eq!(data.data[0].files[0].segments.len(), 3);
        assert_eq!(data.data[0].totals.lines.percent, 66.67);
    }

    #[test]
    fn test_region_extraction() {
        let json = r#"{
            "data": [{
                "files": [{
                    "filename": "src/lib.rs",
                    "segments": [
                        {"line": 1, "col": 1, "count": 1, "has_count": true, "is_region_entry": true, "is_gap_region": false},
                        {"line": 2, "col": 1, "count": 0, "has_count": true, "is_region_entry": true, "is_gap_region": true},
                        {"line": 3, "col": 1, "count": 0, "has_count": false, "is_region_entry": false, "is_gap_region": false}
                    ]
                }],
                "totals": {"lines": {"count": 3, "covered": 1, "percent": 33.33}, "regions": {"count": 3, "covered": 1, "percent": 33.33}, "functions": {"count": 0, "covered": 0, "percent": 0.0}, "branches": {"count": 0, "covered": 0, "percent": 100.0}}
            }],
            "version": "2",
            "type_": "llvm.coverage.json.export"
        }"#;
        let data = parse_coverage_json(json).unwrap();
        let regions = extract_regions(&data);
        assert!(!regions.is_empty());
        assert!(regions.iter().any(|r| r.is_covered));
    }

    #[test]
    fn test_empty_coverage() {
        let json = r#"{"data": [], "version": "2", "type_": "llvm.coverage.json.export"}"#;
        let data = parse_coverage_json(json).unwrap();
        assert!(data.data.is_empty());
    }

    #[test]
    fn test_totals_display() {
        let totals = Totals {
            lines: CountSummary { count: 100, covered: 80, percent: 80.0 },
            regions: CountSummary { count: 50, covered: 30, percent: 60.0 },
            functions: CountSummary { count: 20, covered: 15, percent: 75.0 },
            branches: CountSummary { count: 40, covered: 20, percent: 50.0 },
        };
        assert!((totals.line_percent() - 80.0).abs() < 0.01);
        assert!((totals.function_percent() - 75.0).abs() < 0.01);
        assert!((totals.branch_percent() - 50.0).abs() < 0.01);
    }
}
