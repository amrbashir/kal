use crate::common::SearchResultItem;
use std::{iter, os::windows::prelude::OsStrExt, path, ptr};

pub fn execute(item: &SearchResultItem, elevated: bool) {
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            ptr::null::<isize>() as _,
            if elevated {
                encode_wide("runas").as_ptr()
            } else {
                ptr::null()
            },
            encode_wide(&item.execution_args[0]).as_ptr(),
            ptr::null(),
            ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL as _,
        )
    };
}
pub fn open_location(item: &SearchResultItem) {
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            ptr::null::<isize>() as _,
            encode_wide("open").as_ptr(),
            encode_wide(
                path::PathBuf::from(&item.execution_args[0])
                    .parent()
                    .unwrap_or_else(|| panic!("Failed to find the location of file"))
                    .to_string_lossy()
                    .into_owned(),
            )
            .as_ptr(),
            ptr::null(),
            ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL as _,
        )
    };
}

fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    string.as_ref().encode_wide().chain(iter::once(0)).collect()
}

pub fn extract_png<P: AsRef<path::Path>>(files: Vec<(P, P)>) -> std::io::Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let (srcs, outs): (Vec<P>, Vec<P>) = files.into_iter().unzip();
    let (srcs, outs) = (
        srcs.into_iter()
            .map(|p| format!(r#""{}""#, p.as_ref().to_string_lossy()))
            .collect::<Vec<_>>(),
        outs.into_iter()
            .map(|p| format!(r#""{}""#, p.as_ref().to_string_lossy()))
            .collect::<Vec<_>>(),
    );

    // TODO: use win32 apis
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
        .spawn()?;
    Ok(())
}
