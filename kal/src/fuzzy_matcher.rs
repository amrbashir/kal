#[derive(Default)]
pub struct Matcher {
    inner: nucleo_matcher::Matcher,
}

impl Matcher {
    pub fn fuzzy_match(&mut self, haystack: &str, needle: &str) -> Option<u16> {
        let mut haystack_buf = Vec::new();
        let mut needle_buf = Vec::new();

        let haystack = nucleo_matcher::Utf32Str::new(haystack, &mut haystack_buf);
        let needle = nucleo_matcher::Utf32Str::new(needle, &mut needle_buf);

        self.inner.fuzzy_match(haystack, needle)
    }
}
