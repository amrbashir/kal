pub mod path;
pub mod shell;

use windows::UI::ViewManagement::*;

pub use self::path::*;
pub use self::shell::*;

pub fn system_accent_color() -> Option<String> {
    let settings = UISettings::new().ok()?;
    let color = settings.GetColorValue(UIColorType::AccentLight2).ok()?;
    let color_rgb = format!("rgba({},{},{},{})", color.R, color.G, color.B, color.A);
    Some(color_rgb)
}

pub trait IteratorExt: Iterator {
    fn collect_non_empty<B: FromIterator<Self::Item>>(self) -> Option<B>;
}

impl<T: Iterator> IteratorExt for T {
    fn collect_non_empty<B: FromIterator<Self::Item>>(self) -> Option<B> {
        let mut peek = self.peekable();
        peek.peek().is_some().then(|| peek.collect())
    }
}

#[cfg(test)]
mod tests {
    use super::IteratorExt;

    #[test]
    fn it_collects_non_empty() {
        let empty = Vec::<u32>::new();
        let non_empty = [1];
        let non_empty2 = [1, 2, 3];

        assert!(empty.iter().collect_non_empty::<Vec<_>>().is_none());
        assert!(non_empty.iter().collect_non_empty::<Vec<_>>().is_some());
        assert_eq!(
            non_empty2
                .into_iter()
                .collect_non_empty::<Vec<_>>()
                .unwrap(),
            vec![1, 2, 3]
        );
    }
}
