// ─── Ternary Integration for Lapce (open-iterator) ───────────────────────────
//
// Tracks editing patterns as ternary signals:
//   +1 (Choose) → new code added
//   -1 (Avoid)  → code deleted
//    0 (Unknown) → idle / unchanged
//
// Classifies coding style into strategy species and routes AI model requests.

use std::collections::HashMap;

/// A ternary editing signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTrit {
    Deleted,  // -1 — code removed
    Idle,     //  0 — no change
    Added,    // +1 — code added
}

impl EditTrit {
    pub fn value(&self) -> i8 {
        match self {
            EditTrit::Deleted => -1,
            EditTrit::Idle => 0,
            EditTrit::Added => 1,
        }
    }
}

/// Tracks editing patterns over time, mapping them to ternary strategies.
pub struct EditingTracker {
    /// File → recent edit signals
    history: HashMap<String, Vec<EditTrit>>,
    /// Max history per file
    window: usize,
}

impl EditingTracker {
    pub fn new(window: usize) -> Self {
        Self {
            history: HashMap::new(),
            window,
        }
    }

    /// Record an edit event for a file
    pub fn record(&mut self, file: &str, trit: EditTrit) {
        let entry = self.history.entry(file.to_string()).or_insert_with(Vec::new);
        entry.push(trit);
        if entry.len() > self.window {
            entry.remove(0);
        }
    }

    /// Get the edit ratio for a file: (added, deleted, idle) as proportions
    pub fn edit_ratio(&self, file: &str) -> (f64, f64, f64) {
        match self.history.get(file) {
            Some(edits) if !edits.is_empty() => {
                let n = edits.len() as f64;
                let added = edits.iter().filter(|e| **e == EditTrit::Added).count() as f64 / n;
                let deleted = edits.iter().filter(|e| **e == EditTrit::Deleted).count() as f64 / n;
                let idle = edits.iter().filter(|e| **e == EditTrit::Idle).count() as f64 / n;
                (added, deleted, idle)
            }
            _ => (0.0, 0.0, 1.0),
        }
    }

    /// Get overall edit entropy across all files
    pub fn entropy(&self) -> f64 {
        let total: usize = self.history.values().map(|v| v.len()).sum();
        if total == 0 {
            return 0.0;
        }

        let counts = [
            self.history.values().flat_map(|v| v.iter()).filter(|e| **e == EditTrit::Added).count(),
            self.history.values().flat_map(|v| v.iter()).filter(|e| **e == EditTrit::Deleted).count(),
            self.history.values().flat_map(|v| v.iter()).filter(|e| **e == EditTrit::Idle).count(),
        ];

        counts.iter().filter(|&&c| c > 0).map(|&c| {
            let p = c as f64 / total as f64;
            -p * p.log2()
        }).sum()
    }
}

/// Coding style species — classify how a developer writes code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleSpecies {
    /// High addition, low deletion — building new code
    Builder,
    /// High deletion, low addition — refactoring aggressively
    Surgeon,
    /// Balanced add/delete — iterative refinement
    Balancer,
    /// Low activity — mostly reading/browsing
    Reader,
    /// Erratic pattern — exploration/experimentation
    Explorer,
}

/// Classifies coding style from edit patterns.
pub struct StyleClassifier {
    tracker: EditingTracker,
}

impl StyleClassifier {
    pub fn new(tracker: EditingTracker) -> Self {
        Self { tracker }
    }

    /// Classify the coding style for a specific file
    pub fn classify(&self, file: &str) -> StyleSpecies {
        let (added, deleted, idle) = self.tracker.edit_ratio(file);

        if idle > 0.7 {
            StyleSpecies::Reader
        } else if added > 0.5 && deleted < 0.2 {
            StyleSpecies::Builder
        } else if deleted > 0.4 && added < 0.2 {
            StyleSpecies::Surgeon
        } else if added > 0.2 && deleted > 0.2 {
            StyleSpecies::Balancer
        } else {
            StyleSpecies::Explorer
        }
    }

    /// Classify overall style across all files
    pub fn classify_overall(&self) -> StyleSpecies {
        let files: Vec<_> = self.tracker.history.keys().cloned().collect();
        if files.is_empty() {
            return StyleSpecies::Reader;
        }

        let species: Vec<StyleSpecies> = files.iter().map(|f| self.classify(f)).collect();
        let counts: HashMap<StyleSpecies, usize> = {
            let mut m = HashMap::new();
            for s in &species {
                *m.entry(*s).or_insert(0) += 1;
            }
            m
        };

        counts.into_iter().max_by_key(|(_, c)| *c).map(|(s, _)| s).unwrap_or(StyleSpecies::Reader)
    }
}

/// Routes AI model requests based on coding style species.
/// Different species get different AI model strategies.
pub struct ModelRouter {
    /// Species → model preference
    preferences: HashMap<StyleSpecies, (String, f64)>, // (model_name, temperature)
}

impl ModelRouter {
    pub fn new() -> Self {
        let mut prefs = HashMap::new();
        prefs.insert(StyleSpecies::Builder, ("creative".to_string(), 0.8));
        prefs.insert(StyleSpecies::Surgeon, ("precise".to_string(), 0.2));
        prefs.insert(StyleSpecies::Balancer, ("balanced".to_string(), 0.5));
        prefs.insert(StyleSpecies::Reader, ("fast".to_string(), 0.3));
        prefs.insert(StyleSpecies::Explorer, ("creative".to_string(), 0.9));
        Self { preferences: prefs }
    }

    /// Get recommended model and temperature for a species
    pub fn route(&self, species: StyleSpecies) -> (&str, f64) {
        self.preferences
            .get(&species)
            .map(|(m, t)| (m.as_str(), *t))
            .unwrap_or(("balanced", 0.5))
    }

    /// Get route for current file
    pub fn route_for_file(&self, classifier: &StyleClassifier, file: &str) -> (&str, f64) {
        self.route(classifier.classify(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_trit_values() {
        assert_eq!(EditTrit::Deleted.value(), -1);
        assert_eq!(EditTrit::Idle.value(), 0);
        assert_eq!(EditTrit::Added.value(), 1);
    }

    #[test]
    fn test_tracker_record() {
        let mut t = EditingTracker::new(100);
        t.record("main.rs", EditTrit::Added);
        t.record("main.rs", EditTrit::Added);
        t.record("main.rs", EditTrit::Idle);
        let (a, d, i) = t.edit_ratio("main.rs");
        assert!((a - 0.667).abs() < 0.05);
        assert!(d < 0.01);
        assert!((i - 0.333).abs() < 0.05);
    }

    #[test]
    fn test_tracker_window() {
        let mut t = EditingTracker::new(3);
        t.record("f.rs", EditTrit::Added);
        t.record("f.rs", EditTrit::Added);
        t.record("f.rs", EditTrit::Added);
        t.record("f.rs", EditTrit::Deleted); // pushes out first
        let (a, _, _) = t.edit_ratio("f.rs");
        assert!(a < 1.0); // not all added anymore
    }

    #[test]
    fn test_classify_builder() {
        let mut t = EditingTracker::new(100);
        for _ in 0..20 {
            t.record("new.rs", EditTrit::Added);
        }
        let c = StyleClassifier::new(t);
        assert_eq!(c.classify("new.rs"), StyleSpecies::Builder);
    }

    #[test]
    fn test_classify_surgeon() {
        let mut t = EditingTracker::new(100);
        for _ in 0..20 {
            t.record("refactor.rs", EditTrit::Deleted);
        }
        let c = StyleClassifier::new(t);
        assert_eq!(c.classify("refactor.rs"), StyleSpecies::Surgeon);
    }

    #[test]
    fn test_classify_reader() {
        let mut t = EditingTracker::new(100);
        for _ in 0..20 {
            t.record("readme.rs", EditTrit::Idle);
        }
        let c = StyleClassifier::new(t);
        assert_eq!(c.classify("readme.rs"), StyleSpecies::Reader);
    }

    #[test]
    fn test_classify_balancer() {
        let mut t = EditingTracker::new(100);
        for _ in 0..10 {
            t.record("iter.rs", EditTrit::Added);
            t.record("iter.rs", EditTrit::Deleted);
        }
        let c = StyleClassifier::new(t);
        assert_eq!(c.classify("iter.rs"), StyleSpecies::Balancer);
    }

    #[test]
    fn test_model_router() {
        let r = ModelRouter::new();
        let (model, temp) = r.route(StyleSpecies::Builder);
        assert_eq!(model, "creative");
        assert!(temp > 0.7);
    }

    #[test]
    fn test_model_router_file() {
        let mut t = EditingTracker::new(100);
        for _ in 0..10 {
            t.record("surgery.rs", EditTrit::Deleted);
        }
        let c = StyleClassifier::new(t);
        let r = ModelRouter::new();
        let (model, temp) = r.route_for_file(&c, "surgery.rs");
        assert_eq!(model, "precise");
        assert!(temp < 0.3);
    }

    #[test]
    fn test_entropy() {
        let mut t = EditingTracker::new(100);
        for _ in 0..10 {
            t.record("a.rs", EditTrit::Added);
            t.record("a.rs", EditTrit::Deleted);
            t.record("a.rs", EditTrit::Idle);
        }
        let e = t.entropy();
        assert!(e > 1.0); // high entropy = diverse editing
    }
}
