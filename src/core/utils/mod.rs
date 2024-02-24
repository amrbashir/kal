use crate::{SearchResultItem, TEMP_DIR};
use std::{
    iter,
    os::windows::prelude::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
};
use url::Url;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::HWND,
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    },
};

pub mod thread {
    pub fn spawn<F>(f: F) -> std::thread::JoinHandle<()>
    where
        F: FnOnce() -> anyhow::Result<()>,
        F: Send + 'static,
    {
        std::thread::spawn(move || {
            if let Err(e) = f() {
                tracing::error!("{e}");
            }
        })
    }
}

pub fn resolve_env_vars<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut out = PathBuf::new();

    for c in path.as_ref().components() {
        match c {
            std::path::Component::Normal(c) => {
                if let Some(c) = c.to_str() {
                    // %LOCALAPPDATA%
                    if c.starts_with('%') && c.ends_with('%') {
                        let var = c.strip_prefix('%').unwrap().strip_suffix('%').unwrap();
                        if let Ok(value) = std::env::var(var) {
                            out.push(value);
                            continue;
                        }
                    // $Env:LOCALAPPDATA
                    } else if c[..6].to_lowercase() == "$env:" {
                        let var = &c[6..];
                        if let Ok(value) = std::env::var(var) {
                            out.push(value);
                            continue;
                        }
                    // $LOCALAPPDATA
                    } else if c.starts_with('$') {
                        let var = c.strip_prefix('$').unwrap();
                        if let Ok(value) = std::env::var(var) {
                            out.push(value);
                            continue;
                        }
                    }
                }
                out.push(c);
            }
            _ => out.push(c),
        }
    }

    out
}

pub fn execute(app: impl AsRef<std::ffi::OsStr>, elevated: bool) {
    let app = encode_wide(app);
    unsafe {
        ShellExecuteW(
            HWND::default(),
            if elevated {
                w!("runas")
            } else {
                PCWSTR::null()
            },
            PCWSTR::from_raw(app.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

pub fn open_path<P: AsRef<Path>>(path: P) {
    let path = encode_wide(path.as_ref());
    unsafe {
        ShellExecuteW(
            HWND::default(),
            w!("open"),
            PCWSTR::from_raw(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
}

pub fn open_url(url: &Url) {
    let url = encode_wide(url.as_str());
    unsafe {
        ShellExecuteW(
            HWND::default(),
            w!("open"),
            PCWSTR::from_raw(url.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
}

pub fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    string.as_ref().encode_wide().chain(iter::once(0)).collect()
}

/// Extract pngs from paths, using powershell
///
/// Possiple failures:
/// - When a path is a directory
pub fn extract_pngs<I>(files: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = SearchResultItem>,
{
    let (srcs, outs): (Vec<_>, Vec<_>) = files
        .into_iter()
        .map(|i| {
            (
                PathBuf::from(i.execution_args.as_str().unwrap_or_default()),
                PathBuf::from(i.icon.data),
            )
        })
        .filter_map(|(s, o)| if o.exists() { None } else { Some((s, o)) })
        .unzip();

    if srcs.is_empty() || outs.is_empty() {
        return Ok(());
    }

    let (srcs, outs) = (
        srcs.into_iter()
            .map(|p| format!(r#""{}""#, dunce::simplified(p.as_ref()).display()))
            .collect::<Vec<_>>(),
        outs.into_iter()
            .map(|p| format!(r#""{}""#, dunce::simplified(p.as_ref()).display()))
            .collect::<Vec<_>>(),
    );

    // TODO: use win32 apis
    let script = format!(
        r#"
Add-Type -AssemblyName System.Drawing;
$Shell = New-Object -ComObject WScript.Shell;
$srcs = @({});
$outs = @({});
$len = $srcs.Length;
for ($i=0; $i -lt $len; $i++) {{
$srcPath = $srcs[$i]
try {{
  $path = $Shell.CreateShortcut($srcPath).TargetPath;
  if ((Test-Path -Path $path -PathType Container) -or ($path -match '.url$')) {{
    $path = $srcPath;
  }}
}} catch {{
  $path = $srcPath;
}}
$icon = $null;
try {{
  $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($path);
}} catch {{
  $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($srcPath);
}}
if ($icon -ne $null) {{
  [void]$icon.ToBitmap().Save($outs[$i], [System.Drawing.Imaging.ImageFormat]::Png);
}}
}}
"#,
        &srcs.join(","),
        &outs.join(",")
    );

    let powershell_path = std::env::var("SYSTEMROOT").map_or_else(
        |_| "powershell.exe".to_string(),
        |p| format!("{p}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"),
    );

    std::process::Command::new(powershell_path)
        .args(["-Command", &script])
        .spawn()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn execute_in_shell<S, P>(
    shell: &Option<S>,
    script: &S,
    working_directory: &Option<P>,
    hidden: &Option<bool>,
    elevated: bool,
) -> anyhow::Result<()>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let script_path = TEMP_DIR.join("kal_temp_script.ps1");
    std::fs::write(&script_path, script.as_ref())?;

    let (shell, shell_args) = {
        let shell = shell.as_ref().map(|s| s.as_ref()).unwrap_or("powershell");
        let mut s = shell.split(' ');
        let shell = s.next().unwrap_or(shell);
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

    cmd.spawn().map(|_| ()).map_err(Into::into)
}
