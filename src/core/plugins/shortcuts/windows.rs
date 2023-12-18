use url::Url;

use crate::{utils, TEMP_DIR};
use std::{path::Path, process::Command};

pub fn open_path<P: AsRef<Path>>(path: P) {
    utils::windows::open_path(path);
}

pub fn open_url(url: &Url) {
    utils::windows::open_url(url);
}

pub fn execute_in_shell<S, P>(
    shell: &Option<S>,
    script: &S,
    working_directory: &Option<P>,
    hidden: &Option<bool>,
    elevated: bool,
) where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let script_path = TEMP_DIR.join("kal_temp_script.ps1");
    let _ = std::fs::write(&script_path, script.as_ref());

    let (shell, shell_args) = {
        let shell = shell.as_ref().map(|s| s.as_ref()).unwrap_or("powershell");
        let mut s = shell.split(' ');
        let shell = s.next().unwrap();
        let mut shell_args = s.collect::<Vec<_>>();
        shell_args.push(script_path.to_str().unwrap());
        (shell, shell_args)
    };

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NoLogo", "-Command"]);

    let mut args = vec!["Start-Process"];
    if hidden.unwrap_or(false) {
        args.extend_from_slice(&["-WindowStyle", "Hidden"]);
    }
    if elevated {
        args.extend_from_slice(&["-Verb", "runas"]);
    }
    args.extend_from_slice(&["-FilePath", shell, "-ArgumentList"]);

    let shell_args = format!(
        "@({})",
        shell_args
            .iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    args.push(&shell_args);

    cmd.args(args);

    if let Some(cwd) = working_directory {
        cmd.current_dir(cwd.as_ref());
    }

    let _ = cmd.spawn();
}

pub fn open_location<P: AsRef<Path>>(path: P) {
    if let Some(parent) = path.as_ref().parent() {
        utils::windows::open_path(parent);
    }
}
