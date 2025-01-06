use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub trait PathExt {
    fn with_extra_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf;
}

impl<T: AsRef<Path>> PathExt for T {
    fn with_extra_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        let path = self.as_ref();
        let extension = extension.as_ref();

        let ext = path.extension().map(|e| e.to_string_lossy());

        match ext {
            Some(ext) => path.with_extension(format!("{ext}.{}", extension.to_string_lossy())),
            None => path.with_extension(extension),
        }
    }
}

/// Resolve environment variables components in a path.
///
/// Resolves the follwing formats:
/// - CMD: `%variable%`
/// - PowerShell: `$Env:variable`
/// - Bash: `$variable`.
pub trait ResolveEnvVars {
    /// Resolve environment variables components in a path.
    ///
    /// Resolves the follwing formats:
    /// - CMD: `%variable%`
    /// - PowerShell: `$Env:variable`
    /// - Bash: `$variable`.
    fn resolve_vars(&self) -> PathBuf;
}

impl<T: AsRef<Path>> ResolveEnvVars for T {
    fn resolve_vars(&self) -> PathBuf {
        let mut out = PathBuf::new();

        for c in self.as_ref().components() {
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
            Path::new("/path/%VAR%/to/dir").resolve_vars(),
            os_path("/path/VALUE/to/dir")
        );

        assert_eq!(
            Path::new("/path/$env:VAR/to/dir").resolve_vars(),
            os_path("/path/VALUE/to/dir")
        );

        assert_eq!(
            Path::new("/path/$EnV:VAR/to/dir").resolve_vars(),
            os_path("/path/VALUE/to/dir")
        );

        assert_eq!(
            Path::new("/path/$VAR/to/dir").resolve_vars(),
            os_path("/path/VALUE/to/dir")
        );

        assert_eq!(
            Path::new("/path/%NONEXISTENTVAR%/to/dir").resolve_vars(),
            os_path("/path/%NONEXISTENTVAR%/to/dir")
        );

        assert_eq!(
            Path::new("/path/$env:NONEXISTENTVAR/to/dir").resolve_vars(),
            os_path("/path/$env:NONEXISTENTVAR/to/dir")
        );

        assert_eq!(
            Path::new("/path/$NONEXISTENTVAR/to/dir").resolve_vars(),
            os_path("/path/$NONEXISTENTVAR/to/dir")
        );
    }
}
