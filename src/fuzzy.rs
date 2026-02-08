use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// Returns indices of matched items in relevance order (best match first).
/// If query is empty, returns all indices in original order.
pub fn fuzzy_filter<T, F>(items: &[T], query: &str, get_text: F) -> Vec<usize>
where
    F: Fn(&T) -> &str,
{
    if query.is_empty() {
        return (0..items.len()).collect();
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    let mut scored: Vec<(usize, u32)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let text = get_text(item);
            let mut buf = Vec::new();
            let haystack = Utf32Str::new(text, &mut buf);
            pattern
                .score(haystack, &mut matcher)
                .map(|score| (i, score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_query_returns_all_indices() {
        let items = vec!["build", "test", "dev", "lint"];
        let result = fuzzy_filter(&items, "", |s| s);
        assert_eq!(result, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_exact_match_scores_highest() {
        let items = vec!["build", "rebuild", "build:prod"];
        let result = fuzzy_filter(&items, "build", |s| s);
        // "build" should be first as exact match
        assert_eq!(result[0], 0);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_partial_match_works() {
        let items = vec!["test", "test:unit", "test:integration", "build"];
        let result = fuzzy_filter(&items, "tst", |s| s);
        // All "test" variants should match "tst"
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        // "build" should not match
        assert!(!result.contains(&3));
    }

    #[test]
    fn test_no_match_returns_empty() {
        let items = vec!["build", "test", "dev"];
        let result = fuzzy_filter(&items, "zzz", |s| s);
        assert_eq!(result, Vec::<usize>::new());
    }

    #[test]
    fn test_case_insensitive() {
        let items = vec!["Build", "TEST", "Dev"];
        let result = fuzzy_filter(&items, "build", |s| s);
        assert_eq!(result[0], 0);
    }

    #[test]
    fn test_substring_matching() {
        let items = vec!["start:dev", "start:prod", "test:start", "build"];
        let result = fuzzy_filter(&items, "start", |s| s);
        // All items with "start" should match
        assert_eq!(result.len(), 3);
        assert!(result.contains(&0));
        assert!(result.contains(&1));
        assert!(result.contains(&2));
    }

    #[test]
    fn test_relevance_ordering() {
        let items = vec!["xbuildx", "build", "rebuilder"];
        let result = fuzzy_filter(&items, "build", |s| s);
        // Exact match should score higher than contained match
        assert_eq!(result[0], 1); // "build" exact
    }

    #[test]
    fn test_with_struct() {
        struct Script {
            name: String,
        }

        let scripts = vec![
            Script {
                name: "test".to_string(),
            },
            Script {
                name: "build".to_string(),
            },
            Script {
                name: "test:unit".to_string(),
            },
        ];

        let result = fuzzy_filter(&scripts, "test", |s| &s.name);
        assert_eq!(result.len(), 2);
        // Both test scripts should match
        assert!(result.contains(&0));
        assert!(result.contains(&2));
    }
}
