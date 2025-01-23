use std::ffi::OsString;
use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::icon::Icon;
use crate::result_item::{Action, IntoResultItem, ResultItem};
use crate::utils::{self, ExpandEnvVars, PathExt, StringExt};

#[derive(Debug)]
pub struct Program {
    pub name: OsString,
    pub path: PathBuf,
    pub icon: PathBuf,
    pub id: String,
}

impl Program {
    pub fn new(path: PathBuf, icons_dir: &Path) -> Self {
        let name = path.file_stem().unwrap_or_default().to_os_string();
        let filename = path.file_name().unwrap_or_default().to_os_string();
        let icon = icons_dir.join(&filename).with_extra_extension("png");
        let id = format!("{}:{}", super::Plugin::NAME, filename.to_string_lossy());
        Self {
            name,
            path,
            icon,
            id,
        }
    }

    fn item(&self, args: &str, score: i64) -> ResultItem {
        let path = self.path.clone();
        let args_ = args.to_string();
        let open = Action::primary(move |_| utils::execute_with_args(&path, &args_, false));

        let path = self.path.clone();
        let args_ = args.to_string();
        let open_elevated =
            Action::open_elevated(move |_| utils::execute_with_args(&path, &args_, true));

        let path = self.path.clone();
        let open_location = Action::open_location(move |_| utils::reveal_item_in_dir(&path));

        let tooltip = format!("{}\n{}", self.name.to_string_lossy(), self.path.display());

        ResultItem {
            id: self.id.as_str().into(),
            icon: Icon::path(self.icon.to_string_lossy()),
            primary_text: self.name.to_string_lossy().into_owned(),
            secondary_text: "Application".into(),
            tooltip: Some(tooltip),
            actions: vec![open, open_elevated, open_location],
            score,
        }
    }
}

impl IntoResultItem for Program {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem> {
        let (query, args) = query.split_args().unwrap_or((query, ""));

        matcher
            .fuzzy_match(&self.name.to_string_lossy(), query)
            .or_else(|| matcher.fuzzy_match(&self.path.to_string_lossy(), query))
            .map(|score| self.item(args, score))
    }
}

pub fn find_all_in_paths<'a>(
    paths: &'a [String],
    extensions: &'a [String],
    icons_dir: &'a Path,
) -> impl Iterator<Item = Program> + use<'a> {
    paths
        .iter()
        .map(ExpandEnvVars::expand_vars)
        .filter_map(|p| filter_path_entries_by_extensions(p, extensions).ok())
        .flatten()
        .map(|e| Program::new(e.path(), icons_dir))
}

fn filter_path_entries_by_extensions<P>(
    path: P,
    extensions: &[String],
) -> anyhow::Result<Vec<std::fs::DirEntry>>
where
    P: AsRef<Path>,
{
    let mut filtered = Vec::new();

    let entries = std::fs::read_dir(path)?;
    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                let filtered_entries = filter_path_entries_by_extensions(entry.path(), extensions)?;
                filtered.extend(filtered_entries);
            } else {
                let path = entry.path();
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if extensions.contains(&extension) {
                    filtered.push(entry);
                }
            }
        }
    }

    Ok(filtered)
}
