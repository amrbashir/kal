use crate::common::{utils, SearchResultItem};
use std::path::Path;

pub fn execute(item: &SearchResultItem, _elevated: bool) {
    utils::windows::open_path(&item.execution_args[0])
}
pub fn open_location(item: &SearchResultItem) {
    utils::windows::open_path(&item.execution_args[0])
}

pub fn extract_png<P: AsRef<Path>>(files: Vec<(P, P)>) {
    utils::windows::extract_pngs(files)
}
