//! Coverage Gap Finder — Topological data analysis for Rust test coverage.
//!
//! Parses rustc/llvm-cov JSON output, builds a simplicial complex from code
//! features (branches, loops, match arms, generics, async blocks, unsafe
//! blocks), computes Betti numbers to surface "holes" in coverage, and
//! prioritizes gaps by churn×complexity.

#![allow(clippy::manual_clamp)]

pub mod parse;
pub mod simplicial;
pub mod report;
pub mod plugin;

pub use parse::{CoverageData, FunctionCoverage, RegionCoverage};
pub use simplicial::{build_feature_complex, CoverageFeature, BettiNumbers};
pub use report::{CoverageGapReport, GapEntry, Priority};
