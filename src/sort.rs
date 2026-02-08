use crate::fuzzy::fuzzy_filter;
use crate::store::recents::{self, RecentEntry};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct SortableScript {
    pub key: String,
    pub name: String,
    pub command: String,
}

/// Returns indices into the original `scripts` slice, in display order.
pub fn sort_scripts(
    scripts: &[SortableScript],
    favorites: &HashSet<String>,
    recents: &[RecentEntry],
    query: &str,
) -> Vec<usize> {
    if query.is_empty() {
        sort_scripts_no_query(scripts, favorites, recents)
    } else {
        sort_scripts_with_query(scripts, favorites, recents, query)
    }
}

fn sort_scripts_no_query(
    scripts: &[SortableScript],
    favorites: &HashSet<String>,
    recents: &[RecentEntry],
) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..scripts.len()).collect();

    // Build recent scores map (higher = more recent/frequent)
    let now = recents::now_ms();
    let mut recent_scores: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
    for entry in recents {
        recent_scores.insert(
            entry.key.as_str(),
            recents::frecency_score(entry.count, entry.last_run, now),
        );
    }

    indices.sort_by(|&a, &b| {
        let script_a = &scripts[a];
        let script_b = &scripts[b];

        let is_fav_a = favorites.contains(&script_a.key);
        let is_fav_b = favorites.contains(&script_b.key);

        // Favorites first
        match (is_fav_a, is_fav_b) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            (true, true) => {
                // Both favorites: alphabetical by name
                return script_a.name.cmp(&script_b.name);
            }
            (false, false) => {}
        }

        // Then by recency
        let score_a = recent_scores
            .get(script_a.key.as_str())
            .copied()
            .unwrap_or(0.0);
        let score_b = recent_scores
            .get(script_b.key.as_str())
            .copied()
            .unwrap_or(0.0);

        match score_b.partial_cmp(&score_a) {
            Some(std::cmp::Ordering::Equal) | None => {}
            Some(ord) => return ord,
        }

        // Finally alphabetical by name
        script_a.name.cmp(&script_b.name)
    });

    indices
}

fn sort_scripts_with_query(
    scripts: &[SortableScript],
    favorites: &HashSet<String>,
    recents: &[RecentEntry],
    query: &str,
) -> Vec<usize> {
    // Get fuzzy-matched indices in relevance order
    let matched = fuzzy_filter(scripts, query, |s| &s.name);

    // Build recent scores map
    let now = recents::now_ms();
    let mut recent_scores: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
    for entry in recents {
        recent_scores.insert(
            entry.key.as_str(),
            recents::frecency_score(entry.count, entry.last_run, now),
        );
    }

    // Stable sort by: relevance (already done by fuzzy_filter), then favorite, then recent
    let mut indices = matched;
    indices.sort_by(|&a, &b| {
        let script_a = &scripts[a];
        let script_b = &scripts[b];

        let is_fav_a = favorites.contains(&script_a.key);
        let is_fav_b = favorites.contains(&script_b.key);

        // Favorites win ties
        match (is_fav_a, is_fav_b) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        // Then recency wins ties
        let score_a = recent_scores
            .get(script_a.key.as_str())
            .copied()
            .unwrap_or(0.0);
        let score_b = recent_scores
            .get(script_b.key.as_str())
            .copied()
            .unwrap_or(0.0);

        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_script(key: &str, name: &str) -> SortableScript {
        SortableScript {
            key: key.to_string(),
            name: name.to_string(),
            command: "echo test".to_string(),
        }
    }

    fn make_recent(key: &str, count: u32, last_used_secs_ago: u64) -> RecentEntry {
        let now = recents::now_ms();
        let secs_ago_ms = last_used_secs_ago * 1000;
        RecentEntry {
            key: key.to_string(),
            count,
            last_run: now - secs_ago_ms,
        }
    }

    #[test]
    fn test_no_query_favorites_first() {
        let scripts = vec![
            make_script("build", "build"),
            make_script("test", "test"),
            make_script("dev", "dev"),
        ];

        let mut favorites = HashSet::new();
        favorites.insert("test".to_string());

        let recents = vec![];

        let result = sort_scripts(&scripts, &favorites, &recents, "");

        // "test" (favorite) should be first
        assert_eq!(result[0], 1);
    }

    #[test]
    fn test_no_query_favorites_alphabetical() {
        let scripts = vec![
            make_script("zebra", "zebra"),
            make_script("alpha", "alpha"),
            make_script("beta", "beta"),
        ];

        let mut favorites = HashSet::new();
        favorites.insert("zebra".to_string());
        favorites.insert("alpha".to_string());

        let recents = vec![];

        let result = sort_scripts(&scripts, &favorites, &recents, "");

        // Both are favorites, should be alphabetical
        assert_eq!(result[0], 1); // alpha
        assert_eq!(result[1], 0); // zebra
        assert_eq!(result[2], 2); // beta (not favorite)
    }

    #[test]
    fn test_no_query_recents_by_frecency() {
        let scripts = vec![
            make_script("build", "build"),
            make_script("test", "test"),
            make_script("dev", "dev"),
        ];

        let recents = vec![
            make_recent("build", 5, 100), // count=5, 100s ago
            make_recent("test", 10, 10),  // count=10, 10s ago -> highest score
            make_recent("dev", 3, 50),    // count=3, 50s ago -> lowest score
        ];

        let favorites = HashSet::new();

        let result = sort_scripts(&scripts, &favorites, &recents, "");

        // Order by frecency: test (highest), build (medium count), dev (lowest)
        assert_eq!(result[0], 1); // test - highest frecency
        assert_eq!(result[1], 0); // build - count factor dominates
        assert_eq!(result[2], 2); // dev - lowest frecency
    }

    #[test]
    fn test_no_query_non_recents_alphabetical() {
        let scripts = vec![
            make_script("zebra", "zebra"),
            make_script("alpha", "alpha"),
            make_script("beta", "beta"),
        ];

        let recents = vec![];
        let favorites = HashSet::new();

        let result = sort_scripts(&scripts, &favorites, &recents, "");

        // All should be alphabetical
        assert_eq!(result[0], 1); // alpha
        assert_eq!(result[1], 2); // beta
        assert_eq!(result[2], 0); // zebra
    }

    #[test]
    fn test_with_query_relevance_priority() {
        let scripts = vec![
            make_script("test", "test"),           // exact match
            make_script("test:unit", "test:unit"), // prefix match
            make_script("build", "build"),         // no match
        ];

        let favorites = HashSet::new();
        let recents = vec![];

        let result = sort_scripts(&scripts, &favorites, &recents, "test");

        // Should match both test scripts, not build
        assert_eq!(result.len(), 2);
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(!result.contains(&2));
    }

    #[test]
    fn test_with_query_favorite_breaks_tie() {
        let scripts = vec![
            make_script("test", "test"),
            make_script("test:unit", "test:unit"),
        ];

        let mut favorites = HashSet::new();
        favorites.insert("test:unit".to_string());

        let recents = vec![];

        let result = sort_scripts(&scripts, &favorites, &recents, "test");

        // Both match "test", but "test:unit" is favorite
        assert_eq!(result[0], 1); // test:unit (favorite)
    }

    #[test]
    fn test_with_query_recent_breaks_tie() {
        let scripts = vec![
            make_script("test", "test"),
            make_script("test:unit", "test:unit"),
        ];

        let recents = vec![make_recent("test:unit", 10, 10)];

        let favorites = HashSet::new();

        let result = sort_scripts(&scripts, &favorites, &recents, "test");

        // Both match "test", but "test:unit" is recent
        assert_eq!(result[0], 1); // test:unit (recent)
    }

    #[test]
    fn test_mixed_favorites_and_recents() {
        let scripts = vec![
            make_script("build", "build"),
            make_script("test", "test"),
            make_script("dev", "dev"),
            make_script("lint", "lint"),
        ];

        let mut favorites = HashSet::new();
        favorites.insert("lint".to_string());

        let recents = vec![make_recent("test", 10, 10), make_recent("dev", 5, 50)];

        let result = sort_scripts(&scripts, &favorites, &recents, "");

        // Order: lint (favorite), test (high frecency), dev (medium), build (none)
        assert_eq!(result[0], 3); // lint
        assert_eq!(result[1], 1); // test
        assert_eq!(result[2], 2); // dev
        assert_eq!(result[3], 0); // build
    }

    #[test]
    fn test_empty_scripts() {
        let scripts: Vec<SortableScript> = vec![];
        let favorites = HashSet::new();
        let recents = vec![];

        let result = sort_scripts(&scripts, &favorites, &recents, "");
        assert_eq!(result, Vec::<usize>::new());
    }

    #[test]
    fn test_query_no_matches() {
        let scripts = vec![make_script("build", "build"), make_script("test", "test")];

        let favorites = HashSet::new();
        let recents = vec![];

        let result = sort_scripts(&scripts, &favorites, &recents, "zzz");
        assert_eq!(result, Vec::<usize>::new());
    }
}
