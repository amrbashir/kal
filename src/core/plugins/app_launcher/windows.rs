use crate::common_types::SearchResultItem;
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

fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    string.as_ref().encode_wide().chain(iter::once(0)).collect()
}

pub fn extract_png<P: AsRef<path::Path>>(_src: &P, _out: &P) -> std::io::Result<()> {
    Ok(())
}
