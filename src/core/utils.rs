use crate::TEMP_DIR;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use url::Url;
use windows::{
    core::{w, IntoParam, HSTRING, PCWSTR},
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, HWND},
        System::Com::CoInitialize,
        UI::{
            Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems},
            WindowsAndMessaging::{SHOW_WINDOW_CMD, SW_HIDE, SW_SHOWNORMAL},
        },
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

#[allow(non_snake_case)]
#[inline]
pub unsafe fn ShellExecuteW<P0, P1, P2, P3, P4>(
    hwnd: P0,
    lpoperation: P1,
    lpfile: P2,
    lpparameters: P3,
    lpdirectory: P4,
    nshowcmd: SHOW_WINDOW_CMD,
) -> anyhow::Result<()>
where
    P0: IntoParam<HWND>,
    P1: IntoParam<PCWSTR>,
    P2: IntoParam<PCWSTR>,
    P3: IntoParam<PCWSTR>,
    P4: IntoParam<PCWSTR>,
{
    let hr = windows::Win32::UI::Shell::ShellExecuteW(
        hwnd,
        lpoperation,
        lpfile,
        lpparameters,
        lpdirectory,
        nshowcmd,
    );

    if hr.0 > 32 {
        Ok(())
    } else {
        Err(windows::core::Error::from_win32().into())
    }
}
pub fn execute(app: impl AsRef<std::ffi::OsStr>, elevated: bool) -> anyhow::Result<()> {
    let app = HSTRING::from(app.as_ref());
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
        )
        .map(|_| ())
    }
}

pub fn open_path<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let path = HSTRING::from(path.as_ref());
    unsafe {
        ShellExecuteW(
            HWND::default(),
            w!("open"),
            PCWSTR::from_raw(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
        .map(|_| ())
        .map_err(Into::into)
    }
}

pub fn open_url(url: &Url) -> anyhow::Result<()> {
    let url = HSTRING::from(url.as_str());
    unsafe {
        ShellExecuteW(
            HWND::default(),
            w!("open"),
            PCWSTR::from_raw(url.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
        .map(|_| ())
    }
}

struct ITEMIDLISTPtr(*const windows::Win32::UI::Shell::Common::ITEMIDLIST);
impl Drop for ITEMIDLISTPtr {
    fn drop(&mut self) {
        unsafe { ILFree(Some(self.0)) }
    }
}

pub fn reveal_in_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let _ = unsafe { CoInitialize(None) };

    let path = path.as_ref();

    let Some(dir) = path.parent() else {
        anyhow::bail!("{} doesn't have a parent", path.display());
    };

    let dir = HSTRING::from(dir);
    let dir_item = unsafe { ILCreateFromPathW(PCWSTR::from_raw(dir.as_ptr())) };
    let dir_item = ITEMIDLISTPtr(dir_item);

    let path = HSTRING::from(path);
    let path_item = unsafe { ILCreateFromPathW(PCWSTR::from_raw(path.as_ptr())) };
    let path_item = ITEMIDLISTPtr(path_item);

    unsafe {
        if let Err(e) = SHOpenFolderAndSelectItems(dir_item.0, Some(&[path_item.0]), 0) {
            if e.code().0 == ERROR_FILE_NOT_FOUND.0 as i32 {
                ShellExecuteW(
                    HWND::default(),
                    w!("open"),
                    PCWSTR::from_raw(dir.as_ptr()),
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                )
                .map(|_| ())?;
            } else {
                return Err(e.into());
            }
        }
    }

    Ok(())
}

/// Extract pngs from paths, using powershell
///
/// Possiple failures:
/// - When a path is a directory
pub fn extract_pngs<I>(files: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = (PathBuf, PathBuf)>,
{
    let (srcs, outs): (Vec<_>, Vec<_>) = files.into_iter().unzip();

    let (srcs, outs) = (
        srcs.into_iter()
            .map(|p| format!(r#""{}""#, dunce::simplified(&p).display()))
            .collect::<Vec<_>>(),
        outs.into_iter()
            .map(|p| format!(r#""{}""#, dunce::simplified(&p).display()))
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
    cwd: &Option<P>,
    hidden: &Option<bool>,
    elevated: bool,
) -> anyhow::Result<()>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let (shell, args) = {
        let s = shell.as_ref().map(|s| s.as_ref());
        let s = s.unwrap_or("powershell -Command");
        s.split_once(' ').unwrap_or((s, ""))
    };

    let script_path = TEMP_DIR.join("kal_temp_script.ps1");
    std::fs::write(&script_path, script.as_ref())?;

    let args = format!("{args} {}", script_path.display());

    unsafe {
        let shell = HSTRING::from(shell);
        let args = HSTRING::from(args);
        let cwd = cwd.as_ref().map(|cwd| HSTRING::from(cwd.as_ref()));

        ShellExecuteW(
            HWND::default(),
            if elevated {
                w!("runas")
            } else {
                PCWSTR::null()
            },
            PCWSTR::from_raw(shell.as_ptr()),
            PCWSTR::from_raw(args.as_ptr()),
            cwd.map(|cwd| PCWSTR::from_raw(cwd.as_ptr()))
                .unwrap_or_else(PCWSTR::null),
            if hidden.unwrap_or(false) {
                SW_HIDE
            } else {
                SW_SHOWNORMAL
            },
        )
    }
}

pub fn resolve_env_vars<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut out = PathBuf::new();

    for c in path.as_ref().components() {
        match c {
            std::path::Component::Normal(c) => {
                let bytes = c.as_encoded_bytes();
                // %LOCALAPPDATA%
                if bytes[0] == b'%' && bytes[bytes.len() - 1] == b'%' {
                    let var = &bytes[1..bytes.len() - 1];
                    let var = unsafe { OsStr::from_encoded_bytes_unchecked(var) };
                    if let Ok(value) = std::env::var(var) {
                        out.push(value);
                        continue;
                    }
                } else {
                    // $Env:LOCALAPPDATA
                    let prefix = &bytes[..5.min(bytes.len())];
                    let prefix = unsafe { OsStr::from_encoded_bytes_unchecked(prefix) };
                    if prefix.to_ascii_lowercase() == "$env:" {
                        let var = &bytes[5..];
                        let var = unsafe { OsStr::from_encoded_bytes_unchecked(var) };
                        if let Ok(value) = std::env::var(var) {
                            out.push(value);
                            continue;
                        }
                    // $LOCALAPPDATA
                    } else if bytes[0] == b'$' {
                        let var = &bytes[1..];
                        let var = unsafe { OsStr::from_encoded_bytes_unchecked(var) };
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

#[cfg(test)]
mod tests {

    use super::*;

    fn os_path<P: AsRef<Path>>(p: P) -> PathBuf {
        p.as_ref().components().collect::<PathBuf>()
    }

    #[test]
    fn resolves_env_vars() {
        let var = "VAR";
        let val = "VALUE";
        std::env::set_var(var, val);

        assert_eq!(
            resolve_env_vars("/path/%VAR%/to/dir"),
            os_path("/path/VALUE/to/dir")
        );

        assert_eq!(
            resolve_env_vars("/path/$env:VAR/to/dir"),
            os_path("/path/VALUE/to/dir")
        );

        assert_eq!(
            resolve_env_vars("/path/$EnV:VAR/to/dir"),
            os_path("/path/VALUE/to/dir")
        );

        assert_eq!(
            resolve_env_vars("/path/$VAR/to/dir"),
            os_path("/path/VALUE/to/dir")
        );

        assert_eq!(
            resolve_env_vars("/path/%NONEXISTENTVAR%/to/dir"),
            os_path("/path/%NONEXISTENTVAR%/to/dir")
        );

        assert_eq!(
            resolve_env_vars("/path/$env:NONEXISTENTVAR/to/dir"),
            os_path("/path/$env:NONEXISTENTVAR/to/dir")
        );

        assert_eq!(
            resolve_env_vars("/path/$NONEXISTENTVAR/to/dir"),
            os_path("/path/$NONEXISTENTVAR/to/dir")
        );
    }
}
