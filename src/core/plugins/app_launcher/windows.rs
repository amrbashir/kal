use crate::{common::SearchResultItem, utils};
use std::path::PathBuf;

pub fn execute(item: &SearchResultItem, elevated: bool) {
    utils::windows::execute(item.execution_args.as_str().unwrap(), elevated)
}

pub fn open_location(item: &SearchResultItem) {
    if let Some(parent) = PathBuf::from(&item.execution_args.as_str().unwrap()).parent() {
        utils::windows::open_path(parent)
    }
}

pub fn extract_png<I>(items: I)
where
    I: IntoIterator<Item = SearchResultItem>,
{
    utils::windows::extract_pngs(items)
}
