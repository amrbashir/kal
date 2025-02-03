use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kal_plugin::{Action, Icon, IntoResultItem, ResultItem};
use kal_utils::{ExpandEnvVars, StringExt};

use super::App;

#[derive(Debug)]
pub struct Program {
    pub name: OsString,
    pub path: PathBuf,
    pub id: String,
    pub description: String,
}

impl Program {
    pub fn new(path: PathBuf) -> Self {
        let name = path.file_stem().unwrap_or_default().to_os_string();
        let filename = path.file_name().unwrap_or_default().to_os_string();
        let id = format!("{}:{}", super::Plugin::NAME, filename.to_string_lossy());

        let mut description = String::from("Application");

        #[cfg(windows)]
        if path.extension() == Some(OsStr::new("lnk")) {
            if let Ok(target) = kal_utils::resolve_shortcut_target(&path) {
                description = get_app_type(&target).description().into();
            }
        }

        Self {
            name,
            path,
            id,
            description,
        }
    }

    fn item(&self, args: &str, score: u16) -> ResultItem {
        let path = self.path.clone();
        let args_ = args.to_string();
        let open =
            Action::primary(move |_| kal_utils::execute_with_args(&path, &args_, false, false));

        let path = self.path.clone();
        let args_ = args.to_string();
        let open_elevated = Action::open_elevated(move |_| {
            kal_utils::execute_with_args(&path, &args_, true, false)
        });

        let path = self.path.clone();
        let open_location = Action::open_location(move |_| kal_utils::reveal_item_in_dir(&path));

        let tooltip = format!("{}\n{}", self.name.to_string_lossy(), self.path.display());

        ResultItem {
            id: self.id.as_str().into(),
            icon: Icon::extract_path(self.path.to_string_lossy()),
            primary_text: self.name.to_string_lossy().into_owned(),
            secondary_text: self.description.clone(),
            tooltip: Some(tooltip),
            actions: vec![open, open_elevated, open_location],
            score,
        }
    }
}

impl IntoResultItem for Program {
    fn fuzzy_match(
        &self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> Option<ResultItem> {
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
) -> impl Iterator<Item = Program> + use<'a> {
    paths
        .iter()
        .map(ExpandEnvVars::expand_vars)
        .map(|p| read_dir_filtered_by_exts(p, extensions))
        .flatten()
        .flatten()
        .map(|e| Program::new(e.path()))
}

fn read_dir_filtered_by_exts<P>(path: P, extensions: &[String]) -> anyhow::Result<Vec<fs::DirEntry>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref().to_path_buf();
    let mut filtered = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;

        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                filtered.extend(read_dir_filtered_by_exts(entry.path(), extensions)?);
            } else {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str());
                if extensions.iter().any(|e| Some(e.as_str()) == ext) {
                    filtered.push(entry);
                }
            }
        }
    }

    Ok(filtered)
}

impl super::Plugin {
    pub fn watch_programs(&mut self) -> anyhow::Result<()> {
        use notify::RecursiveMode;
        use notify_debouncer_mini::DebounceEventResult;

        let apps = self.apps.clone();
        let extensions = self.extensions.clone();

        let dur = Duration::from_secs(1);
        let mut debouncer = notify_debouncer_mini::new_debouncer(dur, move |e| {
            let Ok(events): DebounceEventResult = e else {
                return;
            };

            for event in events {
                let path = event.path;

                tracing::trace!("[AppLauncher] detected a change in {}", path.display());

                let flt = |ext| path.extension() == Some(OsStr::new(ext));
                if extensions.iter().any(flt) {
                    let mut apps = apps.lock().unwrap();
                    if let Some(pos) = apps.iter().position(|app| app.path() == Some(&path)) {
                        tracing::trace!("[AppLauncher] removing {}", apps[pos].name());

                        apps.remove(pos);
                    }

                    if path.exists() {
                        let program = Program::new(path);

                        tracing::trace!("[AppLauncher] Adding {}", program.name.to_string_lossy());

                        apps.push(App::Program(program));
                    }
                }
            }
        })?;

        for path in &self.paths {
            let path = Path::new(path).expand_vars();
            debouncer.watcher().watch(&path, RecursiveMode::Recursive)?;
        }

        self.programs_watcher.replace(debouncer);

        Ok(())
    }
}

enum ProgramType {
    Win32Application,
    ShortcutApplication,
    ApprefApplication,
    InternetShortcutApplication,
    GenericFile,
}

impl ProgramType {
    const fn description(&self) -> &str {
        match self {
            ProgramType::Win32Application
            | ProgramType::ShortcutApplication
            | ProgramType::ApprefApplication => "Application",
            ProgramType::InternetShortcutApplication => "Internet shortcut application",
            ProgramType::GenericFile => "File",
        }
    }
}

fn get_app_type(path: &Path) -> ProgramType {
    // taken from https://github.com/microsoft/PowerToys/blob/5fe761949fb92ba3ec60d5a41f1803aa845ba488/src/modules/launcher/Plugins/Microsoft.Plugin.Program/Programs/Win32Program.cs#L96
    const EXE_EXTENSIONS: &[&str] = &[
        "exe", "bat", "bin", "com", "cpl", "msc", "msi", "cmd", "ps1", "job", "msp", "mst", "sct",
        "ws", "wsh", "wsf",
    ];
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return ProgramType::GenericFile;
    };

    match ext {
        "lnk" => ProgramType::ShortcutApplication,
        "appref-ms" => ProgramType::ApprefApplication,
        "url" => ProgramType::InternetShortcutApplication,
        ext if EXE_EXTENSIONS.contains(&ext) => ProgramType::Win32Application,
        _ => ProgramType::GenericFile,
    }
}
