pub mod icon;
pub mod path;
pub mod shell;
pub mod thread;

pub use icon::*;
pub use path::*;
pub use shell::*;

pub trait IteratorExt: Iterator {
    fn collect_non_empty<B: FromIterator<Self::Item>>(self) -> Option<B>;
}

impl<T: Iterator> IteratorExt for T {
    fn collect_non_empty<B: FromIterator<Self::Item>>(self) -> Option<B> {
        let mut peek = self.peekable();
        peek.next().is_some().then(|| peek.collect())
    }
}

#[cfg(test)]
mod tests {
    use super::IteratorExt;

    #[test]
    fn it_collects_non_empty() {
        let empty = Vec::<u32>::new();
        let non_empty = [1];

        assert!(empty.iter().collect_non_empty::<Vec<_>>().is_none());
        assert!(non_empty.iter().collect_non_empty::<Vec<_>>().is_some());
    }
}
