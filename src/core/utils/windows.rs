use std::{
    iter,
    os::windows::prelude::OsStrExt,
    path::{Path, PathBuf},
    ptr,
};
use url::Url;
use windows_sys::{
    w,
    Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
};

use crate::common::SearchResultItem;

pub fn execute(app: impl AsRef<std::ffi::OsStr>, elevated: bool) {
    unsafe {
        ShellExecuteW(
            ptr::null::<isize>() as _,
            if elevated { w!("runas") } else { ptr::null() },
            encode_wide(app).as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL as _,
        )
    };
}

pub fn open_path<P: AsRef<Path>>(path: P) {
    let path = encode_wide(path.as_ref().to_string_lossy().into_owned());
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            ptr::null::<isize>() as _,
            w!("open"),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL as _,
        )
    };
}

pub fn open_url(url: &Url) {
    let url = encode_wide(url.as_str());
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            ptr::null::<isize>() as _,
            w!("open"),
            url.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL as _,
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
pub fn extract_pngs<I>(files: I)
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
        return;
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

    if let Err(e) = std::process::Command::new(powershell_path)
        .args(["-Command", &script])
        .spawn()
    {
        tracing::error!("Failed to extract icons: {e}");
    }
}
