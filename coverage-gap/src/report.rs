//! Coverage gap report: human-readable analysis of topological blind spots.

use std::fmt;

use crate::parse::Totals;
use crate::simplicial::{BettiNumbers, CoverageFeature};

/// Priority ranking for a coverage gap.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Critical: untested unsafe or high-churn code.
    Critical,
    /// High: untested async or generic-heavy code.
    High,
    /// Medium: branches/loops without tests.
    Medium,
    /// Low: minor or edge-case gaps.
    Low,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Critical => write!(f, "🔴 Critical"),
            Priority::High => write!(f, "🟠 High"),
            Priority::Medium => write!(f, "🟡 Medium"),
            Priority::Low => write!(f, "🟢 Low"),
        }
    }
}

/// Individual gap entry in the report.
#[derive(Debug, Clone)]
pub struct GapEntry {
    /// Feature location (file:line).
    pub location: String,
    /// What kind of feature.
    pub feature_type: String,
    /// Why it's uncovered.
    pub reason: String,
    /// How many Betti dimensions this contributes to.
    pub betti_contribution: usize,
    /// Priority ranking.
    pub priority: Priority,
}

/// Full coverage gap analysis report.
#[derive(Debug, Clone)]
pub struct CoverageGapReport {
    /// Overall line coverage percentage.
    pub line_coverage_pct: f64,
    /// Overall function coverage percentage.
    pub function_coverage_pct: f64,
    /// Overall branch coverage percentage.
    pub branch_coverage_pct: f64,
    /// Betti numbers from topological analysis.
    pub betti: BettiNumbers,
    /// Number of features analyzed.
    pub total_features: usize,
    /// Number of covered features.
    pub covered_features: usize,
    /// Number of uncovered features.
    pub uncovered_features: usize,
    /// Sorted list of individual gaps.
    pub gaps: Vec<GapEntry>,
}

impl CoverageGapReport {
    /// Create a report from coverage data and topological analysis.
    pub fn new(
        totals: &Totals,
        betti: BettiNumbers,
        features: &[CoverageFeature],
        gaps: Vec<GapEntry>,
    ) -> Self {
        let total = features.len();
        let covered = features.iter().filter(|f| f.covered).count();
        let uncovered = total - covered;

        CoverageGapReport {
            line_coverage_pct: totals.line_percent(),
            function_coverage_pct: totals.function_percent(),
            branch_coverage_pct: totals.branch_percent(),
            betti,
            total_features: total,
            covered_features: covered,
            uncovered_features: uncovered,
            gaps,
        }
    }

    /// Overall gap score: higher = more blind spots.
    pub fn gap_score(&self) -> f64 {
        let mut score = 0.0;
        // Uncovered features
        if self.total_features > 0 {
            score += (self.uncovered_features as f64 / self.total_features as f64) * 50.0;
        }
        // Betti weight
        score += (self.betti.beta_1 as f64) * 15.0;
        score += (self.betti.beta_2 as f64) * 10.0;
        // Inverse coverage bonus
        score += (100.0 - self.line_coverage_pct) * 0.3;
        score
    }

    /// Render a human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            concat!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
                " Coverage Gap Report\n",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
                " Lines: {:.1}%  |  Functions: {:.1}%  |  Branches: {:.1}%\n",
                "\n",
                " Topological Analysis — Betti Numbers:\n",
                "   {}\n",
                "\n",
                " Features Analyzed: {} ({} covered, {} uncovered)\n",
                " Gap Score: {:.1}\n",
                "\n",
                " Top Prioritized Gaps:\n",
                "{}\n",
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n",
            ),
            self.line_coverage_pct,
            self.function_coverage_pct,
            self.branch_coverage_pct,
            self.betti,
            self.total_features,
            self.covered_features,
            self.uncovered_features,
            self.gap_score(),
            self.render_gaps(10),
        )
    }

    /// Render prioritized gaps (top N).
    pub fn render_gaps(&self, max_gaps: usize) -> String {
        if self.gaps.is_empty() {
            return "   ✅ No significant coverage gaps detected!\n".to_string();
        }

        let mut sorted = self.gaps.clone();
        sorted.sort_by_key(|g| {
            (
                match g.priority {
                    Priority::Critical => 0,
                    Priority::High => 1,
                    Priority::Medium => 2,
                    Priority::Low => 3,
                },
                std::cmp::Reverse(g.betti_contribution),
            )
        });

        sorted
            .iter()
            .take(max_gaps)
            .enumerate()
            .map(|(i, g)| {
                format!(
                    "   {}. {} | {} — {}\n      {}\n",
                    i + 1,
                    g.priority,
                    g.location,
                    g.feature_type,
                    g.reason,
                )
            })
            .collect::<Vec<_>>()
            .concat()
    }
}

impl fmt::Display for CoverageGapReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Create gap entries from features with Betti-aware priority.
pub fn build_gap_entries(
    features: &[CoverageFeature],
    betti: &BettiNumbers,
) -> Vec<GapEntry> {
    let mut gaps = Vec::new();

    for feat in features {
        if feat.covered {
            continue;
        }

        let feature_type = match feat.vector {
            [0, 0, 0, 0, 0, 0] => "plain code",
            [_, 0, 0, 0, 0, 0] => "branches",
            [0, _, 0, 0, 0, 0] => "loops",
            [0, 0, _, 0, 0, 0] => "match arms",
            [0, 0, 0, _, 0, 0] => "generics",
            [_, _, _, _, 1, 0] => "async",
            [_, _, _, _, _, 1] => "unsafe",
            _ => "complex feature",
        };

        let reason = match feature_type {
            "unsafe" => "🔴 Unsafe block without test coverage — undefined behavior risk",
            "async" => "🟠 Async code uncovered — potential silent failure in error paths",
            "generics" => "🟡 Generic code untested — may have type-level bugs",
            "branches" => "🟡 Branch coverage missing — untested code paths",
            "loops" => "🟡 Loop uncovered — edge cases in iteration",
            "match arms" => "🟡 Match arms not fully covered — missed patterns",
            _ => "Code region not covered by any test",
        };

        let (priority, betti_dims) = match feature_type {
            "unsafe" => (Priority::Critical, betti.beta_1.max(1)),
            "async" => (Priority::High, 1),
            "generics" => (Priority::High, 1),
            _ => (Priority::Medium, 0),
        };

        gaps.push(GapEntry {
            location: feat.location.clone(),
            feature_type: feature_type.to_string(),
            reason: reason.to_string(),
            betti_contribution: betti_dims,
            priority,
        });
    }

    gaps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{CountSummary, Totals};
    use crate::simplicial::{BettiNumbers, classify_feature};

    fn sample_totals() -> Totals {
        Totals {
            lines: CountSummary { count: 100, covered: 80, percent: 80.0 },
            regions: CountSummary { count: 50, covered: 30, percent: 60.0 },
            functions: CountSummary { count: 20, covered: 15, percent: 75.0 },
            branches: CountSummary { count: 40, covered: 20, percent: 50.0 },
        }
    }

    fn sample_features() -> Vec<CoverageFeature> {
        vec![
            classify_feature("src/main.rs:10".into(), true, false, false, false, false, false, 1, true),
            classify_feature("src/main.rs:20".into(), false, true, false, false, false, false, 1, true),
            classify_feature("src/unsafe.rs:5".into(), true, false, false, false, false, true, 0, false),
            classify_feature("src/async.rs:10".into(), true, false, false, false, true, false, 1, false),
            classify_feature("src/generic.rs:1".into(), false, false, false, true, false, false, 2, false),
        ]
    }

    #[test]
    fn test_report_creation() {
        let betti = BettiNumbers { beta_0: 2, beta_1: 1, beta_2: 0 };
        let features = sample_features();
        let gaps = build_gap_entries(&features, &betti);
        let report = CoverageGapReport::new(&sample_totals(), betti, &features, gaps);

        assert!((report.line_coverage_pct - 80.0).abs() < 0.01);
        assert_eq!(report.total_features, 5);
        assert_eq!(report.covered_features, 2);
        assert_eq!(report.uncovered_features, 3);
        assert!(report.gap_score() > 0.0);
    }

    #[test]
    fn test_priority_ordering() {
        // Lower discriminant = higher priority (Critical=0, High=1, Medium=2, Low=3)
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Medium);
        assert!(Priority::Medium < Priority::Low);
    }

    #[test]
    fn test_gap_priority() {
        let betti = BettiNumbers { beta_0: 1, beta_1: 1, beta_2: 0 };
        let features = sample_features();
        let gaps = build_gap_entries(&features, &betti);

        let unsafe_gap = gaps.iter().find(|g| g.feature_type == "unsafe").unwrap();
        assert_eq!(unsafe_gap.priority, Priority::Critical);

        let async_gap = gaps.iter().find(|g| g.feature_type == "async").unwrap();
        assert_eq!(async_gap.priority, Priority::High);

        let generic_gap = gaps.iter().find(|g| g.feature_type == "generics").unwrap();
        assert_eq!(generic_gap.priority, Priority::High);
    }

    #[test]
    fn test_empty_report() {
        let betti = BettiNumbers { beta_0: 0, beta_1: 0, beta_2: 0 };
        let report = CoverageGapReport::new(
            &Totals::default(),
            betti,
            &[],
            vec![],
        );
        assert_eq!(report.total_features, 0);
        // With 0 coverage and 0 features, gap_score = inverse coverage bonus only
        assert!((report.gap_score() - 30.0).abs() < 0.001);
        let summary = report.summary();
        assert!(summary.contains("Gap Score"));
    }

    #[test]
    fn test_report_display() {
        let betti = BettiNumbers { beta_0: 2, beta_1: 1, beta_2: 0 };
        let features = sample_features();
        let gaps = build_gap_entries(&features, &betti);
        let report = CoverageGapReport::new(&sample_totals(), betti, &features, gaps);

        let output = report.to_string();
        assert!(output.contains("Betti Numbers"));
        assert!(output.contains("Gap Score"));
        assert!(output.contains("Critical"));
        assert!(output.contains("unsafe"));
    }
}
