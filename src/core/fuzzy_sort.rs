/// Sorts a vector of structs by field
macro_rules! fuzzy_sort {
    ($items:ident, $key:ident, $query:ident) => {
        let matcher = ::fuzzy_matcher::skim::SkimMatcherV2::default();
        let mut si = Vec::new();
        for item in $items {
            si.push((
                ::fuzzy_matcher::FuzzyMatcher::fuzzy_match(&matcher, &item.$key, $query)
                    .unwrap_or_default(),
                item,
            ));
        }
        si.sort_by_cached_key(|i| i.0);
        $items = si.into_iter().rev().map(|i| i.1).collect();
    };
}

pub(crate) use fuzzy_sort;
