use std::{iter, os::windows::prelude::OsStrExt, path::Path, ptr};
use url::Url;
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

pub fn execute(app: impl AsRef<std::ffi::OsStr>, elevated: bool) {
    unsafe {
        ShellExecuteW(
            ptr::null::<isize>() as _,
            if elevated {
                encode_wide("runas").as_ptr()
            } else {
                ptr::null()
            },
            encode_wide(app).as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL as _,
        )
    };
}

pub fn open_path<P: AsRef<Path>>(path: P) {
    let verb = encode_wide("open");
    let path = encode_wide(path.as_ref().to_string_lossy().into_owned());
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            ptr::null::<isize>() as _,
            verb.as_ptr(),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL as _,
        )
    };
}

pub fn open_url(url: &Url) {
    let verb = encode_wide("open");
    let url = encode_wide(url.as_str());
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            ptr::null::<isize>() as _,
            verb.as_ptr(),
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
pub fn extract_pngs<P: AsRef<Path>>(files: Vec<(P, P)>) {
    let (srcs, outs): (Vec<P>, Vec<P>) = files
        .into_iter()
        .filter_map(|(s, o)| {
            if o.as_ref().exists() {
                None
            } else {
                Some((s, o))
            }
        })
        .unzip();

    if srcs.is_empty() || outs.is_empty() {
        return;
    }

    let (srcs, outs) = (
        srcs.into_iter()
            .map(|p| format!(r#""{}""#, p.as_ref().to_string_lossy()))
            .collect::<Vec<_>>(),
        outs.into_iter()
            .map(|p| format!(r#""{}""#, p.as_ref().to_string_lossy()))
            .collect::<Vec<_>>(),
    );

    // TODO: use win32 apis
    if let Err(e) =
        std::process::Command::new("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe")
            .args([
                "-Command",
                &format!(
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
                ),
            ])
            .spawn()
    {
        tracing::error!("Failed to extract icons: {e}");
    }
}
