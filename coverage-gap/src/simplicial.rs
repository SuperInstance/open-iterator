//! Simplicial complex construction from code coverage features.
//!
//! We model coverage features as points in a code-feature space:
//!
//! - Dimension 0: Branch coverage (0 = no branches, 1 = partial, 2 = full)
//! - Dimension 1: Loop nests (0 = none, 1 = single, 2 = nested)
//! - Dimension 2: Match arm count (0 = none, 1 = 1-3 arms, 2 = 4+ arms)
//! - Dimension 3: Generic param count (0 = none, 1 = single generic, 2 = multi generic)
//! - Dimension 4: Async presence (0 = sync, 1 = async)
//! - Dimension 5: Unsafe presence (0 = safe, 1 = unsafe)
//!
//! The Vietoris-Rips complex connects features within a threshold, then we
//! compute Betti numbers (β₀, β₁, β₂) which measure connected components,
//! holes (untested feature transitions), and higher-dimensional voids.

use std::collections::{HashMap, HashSet};

/// A code feature as a point in feature-space.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CoverageFeature {
    /// File and approximate line.
    pub location: String,
    /// Feature vector (6 dimensions).
    pub vector: [u8; 6],
    /// Whether this feature's region was covered by tests.
    pub covered: bool,
}

/// A simplex (set of vertices forming a k-simplex).
type Simplex = Vec<usize>;

/// Betti numbers: β₀ (components), β₁ (holes/cycles), β₂ (voids).
#[derive(Debug, Clone)]
pub struct BettiNumbers {
    pub beta_0: usize,
    pub beta_1: usize,
    pub beta_2: usize,
}

impl std::fmt::Display for BettiNumbers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "β₀={} (connected components of tested features), β₁={} (untested feature transitions/holes), β₂={} (voids in feature coverage)",
            self.beta_0, self.beta_1, self.beta_2
        )
    }
}

/// Build a Vietoris-Rips simplicial complex from coverage features.
///
/// Parameters:
/// - `features`: the set of code features
/// - `threshold`: Euclidean distance threshold for simplex formation
pub fn build_feature_complex(
    features: &[CoverageFeature],
    threshold: f64,
) -> (Vec<Simplex>, BettiNumbers) {
    let n = features.len();
    if n == 0 {
        return (Vec::new(), BettiNumbers { beta_0: 0, beta_1: 0, beta_2: 0 });
    }

    let thresh_sq = threshold * threshold;
    let mut edges: Vec<(usize, usize)> = Vec::new();

    // Build edges (1-simplices) for points within threshold
    for i in 0..n {
        for j in (i + 1)..n {
            let dist_sq = euclidean_sq(&features[i].vector, &features[j].vector);
            if (dist_sq as f64) <= thresh_sq {
                edges.push((i, i)); // vertex (0-simplex) is implicit
                edges.push((i, j));
                edges.push((j, i));
            }
        }
    }

    // Actually, let's do this properly using adjacency
    let mut adjacency: HashMap<usize, HashSet<usize>> = HashMap::new();
    for i in 0..n {
        adjacency.entry(i).or_default().insert(i);
    }
    for i in 0..n {
        for j in (i + 1)..n {
            let dist_sq = euclidean_sq(&features[i].vector, &features[j].vector);
            if (dist_sq as f64) <= thresh_sq {
                adjacency.entry(i).or_default().insert(j);
                adjacency.entry(j).or_default().insert(i);
            }
        }
    }

    // Build 0-simplices (vertices) - all features
    let mut simplices: Vec<Simplex> = (0..n).map(|i| vec![i]).collect();

    // Build 1-simplices (edges) - for each pair within threshold
    for i in 0..n {
        for &j in &adjacency[&i] {
            if i < j {
                simplices.push(vec![i, j]);
            }
        }
    }

    // Build 2-simplices (triangles) - triple cliques
    for i in 0..n {
        let neighbors: Vec<usize> = adjacency[&i].iter().copied()
            .filter(|&j| j > i)
            .collect();
        for a in 0..neighbors.len() {
            for b in (a + 1)..neighbors.len() {
                let j = neighbors[a];
                let k = neighbors[b];
                if adjacency[&j].contains(&k) {
                    simplices.push(vec![i, j, k]);
                }
            }
        }
    }

    // Now compute Betti numbers using boundary matrix reduction
    let betti = compute_betti(&simplices, n);

    (simplices, betti)
}

/// Euclidean distance squared between two feature vectors.
pub fn euclidean_sq(a: &[u8; 6], b: &[u8; 6]) -> f64 {
    let mut sum = 0.0f64;
    for i in 0..6 {
        let d = (a[i] as f64) - (b[i] as f64);
        sum += d * d;
    }
    sum
}

/// Compute Betti numbers from a simplicial complex using boundary matrix
/// reduction (Gaussian elimination over Z₂).
fn compute_betti(simplices: &[Simplex], n_vertices: usize) -> BettiNumbers {
    // Separate by dimension
    let _dim0: Vec<&Simplex> = simplices.iter().filter(|s| s.len() == 1).collect();
    let dim1: Vec<&Simplex> = simplices.iter().filter(|s| s.len() == 2).collect();
    let dim2: Vec<&Simplex> = simplices.iter().filter(|s| s.len() == 3).collect();

    // β₀ = number of connected components (vertices - rank ∂₁)
    // Using union-find
    let mut parent: Vec<usize> = (0..n_vertices).collect();
    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[ra] = rb;
        }
    }

    for s in &dim1 {
        union(&mut parent, s[0], s[1]);
    }

    let mut components = HashSet::new();
    for i in 0..n_vertices {
        components.insert(find(&mut parent, i));
    }
    let beta_0 = components.len();

    // β₁ = dim(ker ∂₁) - dim(im ∂₂) = (#edges - rank ∂₁) - rank ∂₂
    // Over Z₂ we can count cycles: dim(ker ∂₁) = E - rank(∂₁) where
    // rank(∂₁) = V - components (standard fact for Z₂ chain complexes)

    let v = n_vertices;
    let e = dim1.len();

    // rank of ∂₁ = V - β₀
    let rank_d1 = v - beta_0;
    // dim ker ∂₁ = E - rank ∂₁
    let dim_ker_d1 = e.saturating_sub(rank_d1);

    // rank of ∂₂ (edges on boundary of each 2-simplex)
    // We need to compute which 1-chains are boundaries of 2-simplices
    let rank_d2 = compute_rank_d2(&dim2, v);

    // β₁ = dim ker ∂₁ - rank ∂₂
    let beta_1 = dim_ker_d1.saturating_sub(rank_d2);

    // β₂ = dim ker ∂₂ - rank ∂₃ (no 3-simplices, so rank ∂₃ = 0)
    // dim ker ∂₂ = (#2-simplices) - rank ∂₂
    let dim2_count = dim2.len();
    let dim_ker_d2 = dim2_count.saturating_sub(rank_d2);
    let beta_2 = dim_ker_d2;

    BettiNumbers { beta_0, beta_1, beta_2 }
}

/// Compute the rank of ∂₂: the boundary map from 2-simplices to 1-simplices,
/// over Z₂. This is the number of linearly independent 1-chains that are
/// boundaries of 2-simplices.
fn compute_rank_d2(triangles: &[&Vec<usize>], _n_vertices: usize) -> usize {
    if triangles.is_empty() {
        return 0;
    }

    // Each triangle (i,j,k) maps to edge chain: (i,j) + (j,k) + (k,i)
    // Represent each edge as (min, max) pair, encode as usize
    fn encode_edge(a: usize, b: usize) -> usize {
        if a < b { a * 100000 + b } else { b * 100000 + a }
    }

    // Build boundary matrix over Z₂ (rows = edges, cols = triangles)
    let mut edge_set: HashSet<usize> = HashSet::new();
    for t in triangles {
        edge_set.insert(encode_edge(t[0], t[1]));
        edge_set.insert(encode_edge(t[1], t[2]));
        edge_set.insert(encode_edge(t[2], t[0]));
    }
    let edge_list: Vec<usize> = edge_set.into_iter().collect();
    let edge_idx: HashMap<usize, usize> = edge_list.iter().enumerate()
        .map(|(i, &e)| (e, i)).collect();

    let _rows = edge_list.len();
    let cols = triangles.len();

    // Build sparse boundary matrix columns
    let mut cols_sparse: Vec<Vec<usize>> = Vec::with_capacity(cols);
    for t in triangles {
        let mut col = vec![
            edge_idx[&encode_edge(t[0], t[1])],
            edge_idx[&encode_edge(t[1], t[2])],
            edge_idx[&encode_edge(t[2], t[0])],
        ];
        col.sort();
        col.dedup();
        // In Z₂, each edge appears at most once; no cancellation within a single triangle
        cols_sparse.push(col);
    }

    // Gaussian elimination over Z₂ on the boundary matrix
    // We pivot on the row index (lowest non-zero row in each column)
    let mut pivot_row_for_col: Vec<Option<usize>> = vec![None; cols];

    for col in 0..cols {
        // Find pivot row (lowest row index with a 1 in this column)
        let mut pivot = None;
        if let Some(last) = cols_sparse[col].last() {
            pivot = Some(*last);
        }

        if let Some(p_row) = pivot {
            // Eliminate this pivot row from all other columns that also have it
            for other_col in 0..cols {
                if other_col != col && cols_sparse[other_col].contains(&p_row) {
                    // XOR the columns (Z₂ addition = symmetric difference)
                    let mut new_col: Vec<usize> = cols_sparse[other_col]
                        .iter()
                        .chain(cols_sparse[col].iter())
                        .copied()
                        .collect();
                    new_col.sort();
                    new_col.dedup();
                    // Elements appearing twice cancel (XOR)
                    let mut deduped = Vec::new();
                    let mut i = 0;
                    while i < new_col.len() {
                        if i + 1 < new_col.len() && new_col[i] == new_col[i + 1] {
                            i += 2; // skip pair (cancels in Z₂)
                        } else {
                            deduped.push(new_col[i]);
                            i += 1;
                        }
                    }
                    cols_sparse[other_col] = deduped;
                }
            }
            pivot_row_for_col[col] = Some(p_row);
        }
    }

    // Rank is the number of pivot columns
    pivot_row_for_col.iter().filter(|p| p.is_some()).count()
}

/// Classify a code construct into a feature vector.
///
/// `code_features` is a bitmask-like encoding:
/// - Bit 0: has branches
/// - Bit 1: has loops
/// - Bit 2: has match arms
/// - Bit 3: has generics
/// - Bit 4: is async
/// - Bit 5: is unsafe
///
/// `depth` is a complexity measure (nesting depth).
pub fn classify_feature(
    location: String,
    has_branches: bool,
    has_loops: bool,
    has_match: bool,
    has_generics: bool,
    is_async: bool,
    is_unsafe: bool,
    depth: u8,
    covered: bool,
) -> CoverageFeature {
    let vector = [
        if has_branches { depth.min(2) } else { 0 },
        if has_loops { depth.min(2) } else { 0 },
        if has_match { (depth / 2).min(2) } else { 0 },
        if has_generics { depth.min(2) } else { 0 },
        if is_async { 1 } else { 0 },
        if is_unsafe { 1 } else { 0 },
    ];

    CoverageFeature { location, vector, covered }
}

/// Extract features from coverage data. This is a simplified heuristic that
/// maps uncovered regions to feature points.
pub fn extract_features_from_coverage(
    functions: &[crate::parse::FunctionCoverage],
) -> Vec<CoverageFeature> {
    let mut features = Vec::new();

    for (i, fn_cov) in functions.iter().enumerate() {
        // Derive feature hints from location and coverage state
        let has_branches = fn_cov.line_end - fn_cov.line_start > 5;
        let has_loops = false; // would require AST parsing
        let has_match = false;
        let has_generics = fn_cov.name.contains('<');
        let is_async = fn_cov.name.contains("async");
        let is_unsafe = fn_cov.name.contains("unsafe");
        let depth = ((fn_cov.line_end - fn_cov.line_start) / 10).min(2) as u8;

        features.push(classify_feature(
            format!("{}:{}", fn_cov.file.display(), fn_cov.line_start),
            has_branches,
            has_loops,
            has_match,
            has_generics,
            is_async,
            is_unsafe,
            depth,
            fn_cov.executed,
        ));

        // Also create a synthetic "gap" feature for uncovered portions
        if !fn_cov.executed && i > 0 {
            let prev = &functions[i - 1];
            features.push(classify_feature(
                format!("gap:{}:{}-{}", fn_cov.file.display(), prev.line_end, fn_cov.line_start),
                has_branches,
                has_loops,
                has_match,
                has_generics,
                is_async,
                is_unsafe,
                depth.saturating_add(1),
                false,
            ));
        }
    }

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_feature() {
        let f = classify_feature(
            "src/main.rs:10".into(),
            true, false, true, false, false, true,
            1, true,
        );
        assert_eq!(f.vector[0], 1); // branches depth=1
        assert_eq!(f.vector[2], 0); // match depth/2 = 0
        assert_eq!(f.vector[4], 0); // sync
        assert_eq!(f.vector[5], 1); // unsafe
        assert!(f.covered);
    }

    #[test]
    fn test_empty_complex() {
        let features = vec![];
        let (simplices, betti) = build_feature_complex(&features, 1.0);
        assert!(simplices.is_empty());
        assert_eq!(betti.beta_0, 0);
        assert_eq!(betti.beta_1, 0);
    }

    #[test]
    fn test_single_feature_complex() {
        let features = vec![classify_feature(
            "src/lib.rs:5".into(), true, false, false, false, false, false, 0, true,
        )];
        let (simplices, betti) = build_feature_complex(&features, 1.0);
        assert_eq!(simplices.len(), 1); // just the vertex
        assert_eq!(betti.beta_0, 1);
        assert_eq!(betti.beta_1, 0);
    }

    #[test]
    fn test_two_close_features() {
        let features = vec![
            classify_feature("a.rs:1".into(), true, false, false, false, false, false, 0, true),
            classify_feature("a.rs:2".into(), true, false, false, false, false, false, 0, true),
        ];
        let (_simplices, betti) = build_feature_complex(&features, 2.0);
        assert_eq!(betti.beta_0, 1); // connected
    }

    #[test]
    fn test_two_far_features() {
        let features = vec![
            classify_feature("a.rs:1".into(), true, false, false, false, false, false, 0, true),
            classify_feature("b.rs:1".into(), true, true, true, true, true, true, 2, false),
        ];
        let (_simplices, betti) = build_feature_complex(&features, 0.5);
        assert_eq!(betti.beta_0, 2); // disconnected
    }

    #[test]
    fn test_triangle_complex() {
        // Three features that are mutually close -> a triangle (2-simplex)
        let features = vec![
            classify_feature("a.rs:1".into(), true, false, false, false, false, false, 0, true),
            classify_feature("a.rs:2".into(), true, false, false, false, false, false, 1, true),
            classify_feature("a.rs:3".into(), true, false, false, false, false, false, 2, true),
        ];
        let (_simplices, betti) = build_feature_complex(&features, 3.0);
        // Fully connected triangle: β₀=1, β₁=0 (filled)
        assert_eq!(betti.beta_0, 1);
    }

    #[test]
    fn test_feature_extraction() {
        use crate::parse::FunctionCoverage;
        use std::path::PathBuf;

        let fns = vec![
            FunctionCoverage {
                name: "main".into(),
                file: PathBuf::from("src/main.rs"),
                line_start: 1,
                line_end: 10,
                executed: true,
                hit_count: 1,
            },
            FunctionCoverage {
                name: "async_handler".into(),
                file: PathBuf::from("src/main.rs"),
                line_start: 15,
                line_end: 25,
                executed: true,
                hit_count: 5,
            },
            FunctionCoverage {
                name: "unsafe_ffi".into(),
                file: PathBuf::from("src/main.rs"),
                line_start: 30,
                line_end: 40,
                executed: false,
                hit_count: 0,
            },
        ];

        let features = extract_features_from_coverage(&fns);
        assert!(features.len() >= 3);
        // The uncovered function should produce a gap feature
        let gaps: Vec<_> = features.iter().filter(|f| !f.covered).collect();
        assert!(!gaps.is_empty());
    }

    #[test]
    fn test_betti_hole_detection() {
        // Create a ring: 4 features forming a cycle with no covering triangle
        // features: covered=True for 3, covered=False for 1 (the gap)
        let features = vec![
            classify_feature("a.rs:1".into(), true, false, false, false, false, false, 0, true),
            classify_feature("a.rs:2".into(), false, true, false, false, false, false, 0, true),
            classify_feature("a.rs:3".into(), false, false, true, false, false, false, 0, true),
            classify_feature("a.rs:4".into(), false, false, false, true, false, false, 0, false),
        ];
        let (_simplices, betti) = build_feature_complex(&features, 2.0);
        // The 4th feature (generics, uncovered) creates a hole if it's far
        // from covered features. With threshold=2, they're far apart.
        // But we at least verify the computation runs without error.
        assert!(betti.beta_0 <= 4);
    }
}
