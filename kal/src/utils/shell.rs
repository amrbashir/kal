use std::path::Path;

use url::Url;

pub fn execute(app: impl AsRef<std::ffi::OsStr>, elevated: bool) -> anyhow::Result<()> {
    imp::execute(app, elevated)
}

pub fn execute_with_args(
    app: impl AsRef<std::ffi::OsStr>,
    args: impl AsRef<std::ffi::OsStr>,
    elevated: bool,
) -> anyhow::Result<()> {
    imp::execute_with_args(app, args, elevated)
}

pub fn open_url(url: &Url) -> anyhow::Result<()> {
    imp::open_url(url)
}

pub fn open_dir(path: impl AsRef<Path>) -> anyhow::Result<()> {
    imp::open_dir(path)
}

pub fn reveal_in_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    imp::reveal_in_dir(path)
}

pub fn execute_in_shell<S, P>(
    shell: Option<S>,
    script: S,
    cwd: Option<P>,
    hidden: Option<bool>,
    elevated: bool,
) -> anyhow::Result<()>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    imp::execute_in_shell(shell, script, cwd, hidden, elevated)
}

#[cfg(windows)]
mod imp {
    use windows::core::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Com::*;
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    use super::*;

    pub fn execute(app: impl AsRef<std::ffi::OsStr>, elevated: bool) -> anyhow::Result<()> {
        let app = HSTRING::from(app.as_ref());
        unsafe {
            ffi::ShellExecuteW(
                None,
                if elevated {
                    w!("runas")
                } else {
                    PCWSTR::null()
                },
                &app,
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
            .map(|_| ())
        }
    }

    pub fn execute_with_args(
        app: impl AsRef<std::ffi::OsStr>,
        args: impl AsRef<std::ffi::OsStr>,
        elevated: bool,
    ) -> anyhow::Result<()> {
        let app = HSTRING::from(app.as_ref());
        let args = HSTRING::from(args.as_ref());
        unsafe {
            ffi::ShellExecuteW(
                None,
                if elevated {
                    w!("runas")
                } else {
                    PCWSTR::null()
                },
                &app,
                &args,
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
            .map(|_| ())
        }
    }

    pub fn open_url(url: &Url) -> anyhow::Result<()> {
        let url = HSTRING::from(url.as_str());
        unsafe {
            ffi::ShellExecuteW(
                None,
                w!("open"),
                &url,
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        }
    }

    pub fn open_dir(path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        let path = HSTRING::from(path);

        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as _,
            nShow: SW_SHOWNORMAL.0,
            lpVerb: w!("explore"),
            lpClass: w!("folder"),
            lpFile: PCWSTR::from_raw(path.as_ptr()),
            ..unsafe { std::mem::zeroed() }
        };
        unsafe { ShellExecuteExW(&mut info).map_err(Into::into) }
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
                    ffi::ShellExecuteW(
                        None,
                        w!("open"),
                        &dir,
                        PCWSTR::null(),
                        PCWSTR::null(),
                        SW_SHOWNORMAL,
                    )?;
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    pub fn execute_in_shell<S, P>(
        shell: Option<S>,
        script: S,
        cwd: Option<P>,
        hidden: Option<bool>,
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

        let script_path = std::env::temp_dir().join("kal_temp_script.ps1");
        std::fs::write(&script_path, script.as_ref())?;

        let args = format!("{args} {}", script_path.display());

        unsafe {
            let shell = HSTRING::from(shell);
            let args = HSTRING::from(args);
            let cwd = cwd
                .as_ref()
                .map(|cwd| HSTRING::from(cwd.as_ref()))
                .unwrap_or_default();

            ffi::ShellExecuteW(
                None,
                if elevated {
                    w!("runas")
                } else {
                    PCWSTR::null()
                },
                &shell,
                &args,
                &cwd,
                if hidden.unwrap_or(false) {
                    SW_HIDE
                } else {
                    SW_SHOWNORMAL
                },
            )
        }
    }

    #[allow(non_snake_case)]
    mod ffi {
        use super::*;

        pub unsafe fn ShellExecuteW<P1, P2, P3, P4>(
            hwnd: Option<HWND>,
            lpoperation: P1,
            lpfile: P2,
            lpparameters: P3,
            lpdirectory: P4,
            nshowcmd: SHOW_WINDOW_CMD,
        ) -> anyhow::Result<()>
        where
            P1: Param<PCWSTR>,
            P2: Param<PCWSTR>,
            P3: Param<PCWSTR>,
            P4: Param<PCWSTR>,
        {
            let hr = windows::Win32::UI::Shell::ShellExecuteW(
                hwnd,
                lpoperation,
                lpfile,
                lpparameters,
                lpdirectory,
                nshowcmd,
            );

            if hr.0 as isize > 32 {
                Ok(())
            } else {
                Err(windows::core::Error::from_win32().into())
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn execute(app: impl AsRef<std::ffi::OsStr>, elevated: bool) -> anyhow::Result<()> {
        unimplemented!()
    }

    pub fn open_url(url: &Url) -> anyhow::Result<()> {
        unimplemented!()
    }

    pub fn open_dir(path: impl AsRef<Path>) -> anyhow::Result<()> {
        unimplemented!()
    }

    pub fn reveal_in_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
        unimplemented!()
    }

    pub fn execute_in_shell<S, P>(
        shell: Option<S>,
        script: S,
        cwd: Option<P>,
        hidden: Option<bool>,
        elevated: bool,
    ) -> anyhow::Result<()>
    where
        S: AsRef<str>,
        P: AsRef<Path>,
    {
        unimplemented!()
    }
}
