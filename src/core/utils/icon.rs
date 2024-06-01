use std::path::PathBuf;

/// Extract pngs from paths, using powershell
///
/// Possiple failures:
/// - When a path is a directory
pub fn extract_pngs<I>(files: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = (PathBuf, PathBuf)>,
{
    let (srcs, outs): (Vec<_>, Vec<_>) = files.into_iter().unzip();

    let srcs = srcs
        .into_iter()
        .map(|p| format!(r#""{}""#, dunce::simplified(&p).display()))
        .collect::<Vec<_>>();

    let outs = outs
        .into_iter()
        .map(|p| format!(r#""{}""#, dunce::simplified(&p).display()))
        .collect::<Vec<_>>();

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
