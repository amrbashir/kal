use crate::{common::SearchResultItem, utils};
use std::path::{Path, PathBuf};

pub fn execute(item: &SearchResultItem, _elevated: bool) {
    utils::windows::open_path(item.execution_args.as_str().unwrap())
}

pub fn open_location(item: &SearchResultItem) {
    if let Some(parent) = PathBuf::from(&item.execution_args.as_str().unwrap()).parent() {
        utils::windows::open_path(&*parent.to_string_lossy())
    }
}

pub fn extract_png<P: AsRef<Path>>(files: Vec<(P, P)>) {
    utils::windows::extract_pngs(files)
}
