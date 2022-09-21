use crate::{common::SearchResultItem, utils};
use std::path::{Path, PathBuf};

pub fn execute(item: &SearchResultItem, elevated: bool) {
    utils::windows::execute(&item.execution_args.as_str().unwrap(), elevated)
}
pub fn open_location(item: &SearchResultItem) {
    if let Some(parent) = PathBuf::from(&item.execution_args.as_str().unwrap()).parent() {
        utils::windows::open_path(&*parent.to_string_lossy())
    }
}

pub fn extract_png<P: AsRef<Path>>(files: Vec<(P, P)>) {
    utils::windows::extract_pngs(files)
}
