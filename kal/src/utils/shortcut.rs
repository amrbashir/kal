use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows::core::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::*;

pub fn resolve_shortcut_target<P: AsRef<Path>>(path: P) -> anyhow::Result<PathBuf> {
    let path = path.as_ref();
    let path_hstr = HSTRING::from(path);

    let sl: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_ALL) }?;
    let pf = sl.cast::<IPersistFile>()?;

    unsafe { pf.Load(&path_hstr, STGM_READ) }?;

    let mut target_path = [0_u16; 128];
    let mut find_data = WIN32_FIND_DATAW::default();
    unsafe { sl.GetPath(&mut target_path, &mut find_data, 0) }?;

    let nul = target_path.iter().position(|e| *e == 0);

    let target_path = OsString::from_wide(&target_path[..nul.unwrap_or(128)]);

    Ok(PathBuf::from(target_path))
}
