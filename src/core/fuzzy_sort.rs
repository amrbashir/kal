/// Sorts a vectore of
macro_rules! fuzzy_sort {
    ($items:ident, $key:ident, $query:ident) => {
        let mut sorted_items = Vec::new();
        let matcher = ::fuzzy_matcher::skim::SkimMatcherV2::default();
        // let mut sorted_items = Vec::new();
        for item in $items {
            let score = ::fuzzy_matcher::FuzzyMatcher::fuzzy_match(&matcher, &item.$key, $query)
                .unwrap_or_default();

            sorted_items.push((score, item));
        }

        sorted_items.sort_by_key(|i| i.0);
        $items = sorted_items.into_iter().rev().map(|i| i.1).collect();
    };
}

pub(crate) use fuzzy_sort;
