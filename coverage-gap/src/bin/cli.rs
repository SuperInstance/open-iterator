//! Coverage Gap Finder — CLI tool.
//!
//! Usage:
//!   coverage-gap <coverage.json>                     Analyze coverage JSON and print report
//!   coverage-gap --coverage <coverage.json>          Same, explicit
//!   coverage-gap --project <dir>                     Auto-detect coverage.json in project dir
//!   coverage-gap --json                              Output raw JSON report
//!   coverage-gap --threshold <float>                 Set simplicial complex threshold

use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mut coverage_path: Option<PathBuf> = None;
    let mut project_dir: Option<PathBuf> = None;
    let mut json_output = false;
    let mut threshold: Option<f64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--coverage" | "-c" => {
                i += 1;
                coverage_path = Some(PathBuf::from(&args[i]));
            }
            "--project" | "-p" => {
                i += 1;
                project_dir = Some(PathBuf::from(&args[i]));
            }
            "--json" | "-j" => {
                json_output = true;
            }
            "--threshold" | "-t" => {
                i += 1;
                threshold = Some(args[i].parse()?);
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                // Treat as positional: coverage JSON path
                if coverage_path.is_none() && !args[i].starts_with('-') {
                    coverage_path = Some(PathBuf::from(&args[i]));
                }
            }
        }
        i += 1;
    }

    // Resolve coverage file path
    let cov_path = match coverage_path {
        Some(p) => p,
        None => match project_dir {
            Some(dir) => coverage_gap::plugin::find_coverage_json(&dir)
                .ok_or_else(|| anyhow::anyhow!("No coverage.json found in {:?}", dir))?,
            None => {
                // Try current directory
                let cwd = std::env::current_dir()?;
                coverage_gap::plugin::find_coverage_json(&cwd)
                    .ok_or_else(|| anyhow::anyhow!(
                        "No coverage.json found. Run `cargo llvm-cov --json` first.\n\
                         Usage: coverage-gap <coverage.json>"
                    ))?
            }
        },
    };

    if !cov_path.exists() {
        anyhow::bail!("Coverage file not found: {:?}", cov_path);
    }

    if json_output {
        run_json(&cov_path, threshold)?;
    } else {
        run_human(&cov_path, threshold)?;
    }

    Ok(())
}

fn run_human(cov_path: &PathBuf, _threshold: Option<f64>) -> anyhow::Result<()> {
    let workspace_root = cov_path.parent().unwrap_or(std::path::Path::new("."));
    let result = coverage_gap::plugin::run_coverage_gap(cov_path, workspace_root)?;
    println!("{}", result.report);
    Ok(())
}

fn run_json(cov_path: &PathBuf, _threshold: Option<f64>) -> anyhow::Result<()> {
    let workspace_root = cov_path.parent().unwrap_or(std::path::Path::new("."));
    let result = coverage_gap::plugin::run_coverage_gap(cov_path, workspace_root)?;

    let output = serde_json::json!({
        "line_coverage_pct": result.report.line_coverage_pct,
        "function_coverage_pct": result.report.function_coverage_pct,
        "branch_coverage_pct": result.report.branch_coverage_pct,
        "gap_score": result.report.gap_score(),
        "betti": {
            "beta_0": result.report.betti.beta_0,
            "beta_1": result.report.betti.beta_1,
            "beta_2": result.report.betti.beta_2,
        },
        "total_features": result.report.total_features,
        "covered_features": result.report.covered_features,
        "uncovered_features": result.report.uncovered_features,
        "gaps": result.report.gaps.iter().map(|g| {
            serde_json::json!({
                "location": g.location,
                "feature_type": g.feature_type,
                "reason": g.reason,
                "betti_contribution": g.betti_contribution,
                "priority": format!("{}", g.priority),
            })
        }).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_help() {
    println!(concat!(
        "Coverage Gap Finder — Topological test-coverage analysis\n",
        "\n",
        "USAGE:\n",
        "  coverage-gap [FLAGS] [coverage.json]\n",
        "\n",
        "FLAGS:\n",
        "  -c, --coverage <FILE>     Path to llvm-cov JSON output\n",
        "  -p, --project <DIR>       Project directory (auto-detect coverage.json)\n",
        "  -j, --json                Output raw JSON\n",
        "  -t, --threshold <FLOAT>   Simplicial complex distance threshold\n",
        "  -h, --help                Print this help\n",
        "\n",
        "EXAMPLES:\n",
        "  coverage-gap target/llvm-cov/coverage.json\n",
        "  coverage-gap --project .\n",
        "  coverage-gap --coverage cov.json --json\n",
        "\n",
        "GENERATING COVERAGE:\n",
        "  cargo llvm-cov --json > coverage.json\n",
        "  cargo tarpaulina --out Json > coverage.json\n",
    ));
}
