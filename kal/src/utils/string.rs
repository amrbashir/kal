pub trait StringExt: AsRef<str> {
    fn split_args(&self) -> Option<(&str, &str)> {
        let s = self.as_ref();

        if let Some(idx) = s.find(" --") {
            let next_char = s.as_bytes().get(idx + 3).map(|c| *c as char);
            if next_char.map(char::is_whitespace).unwrap_or(true) {
                return Some((&s[..idx], &s[idx + 3..]));
            }
        }

        if let Some(idx) = s.find(" -") {
            let next_char = s.as_bytes().get(idx + 2).map(|c| *c as char);
            if next_char.map(char::is_alphabetic).unwrap_or_default() {
                return Some((&s[..idx], &s[idx + 1..]));
            }
        }

        None
    }
}

impl<T: AsRef<str>> StringExt for T {}

#[cfg(test)]
mod tests {
    use super::StringExt;

    #[test]
    fn it_extracts_args() {
        assert_eq!("program space".split_args(), None);
        assert_eq!("program space -".split_args(), None);
        assert_eq!("program space - args".split_args(), None);
        assert_eq!(
            "program space -d args".split_args(),
            Some(("program space", "-d args"))
        );
        assert_eq!("program space --".split_args(), Some(("program space", "")));
        assert_eq!(
            "program space -- args".split_args(),
            Some(("program space", " args"))
        );
        assert_eq!(
            "program space -- -d args".split_args(),
            Some(("program space", " -d args"))
        );
        assert_eq!("program space ---d args".split_args(), None);
    }
}
