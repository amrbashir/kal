use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

pub trait PathExt {
    #[allow(unused)]
    fn with_extra_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf;

    fn to_hash(&self) -> String;
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

    fn to_hash(&self) -> String {
        let path = self.as_ref();
        let mut hasher = DefaultHasher::default();
        path.hash(&mut hasher);
        hasher.finish().to_string()
    }
}

/// Expand environment variables components in a path.
pub trait ExpandEnvVars {
    /// Expand environment variables components in a path.
    ///
    /// Expands the follwing formats:
    /// - CMD: `%variable%`
    /// - PowerShell: `$Env:variable`
    /// - Bash: `$variable`.
    fn expand_vars(&self) -> PathBuf;
}

impl<T: AsRef<Path>> ExpandEnvVars for T {
    fn expand_vars(&self) -> PathBuf {
        let mut out = PathBuf::new();

        for c in self.as_ref().components() {
            match c {
                std::path::Component::Normal(mut c) => {
                    // Special case for `~` and `$HOME` on Windows, replace with `$Env:USERPROFILE`
                    #[cfg(windows)]
                    if c == OsStr::new("~") || c.eq_ignore_ascii_case("$HOME") {
                        c = OsStr::new("$Env:USERPROFILE");
                    }

                    // Special case for `~` on Unix, replace with `$HOME`
                    #[cfg(not(windows))]
                    if c == OsStr::new("~") {
                        c = OsStr::new("HOME");
                    }

                    let bytes = c.as_encoded_bytes();

                    // %LOCALAPPDATA%
                    let var = if bytes[0] == b'%' && bytes[bytes.len() - 1] == b'%' {
                        Some(&bytes[1..bytes.len() - 1])
                    } else {
                        // prefix length is 5 for $Env: and 1 for $
                        // so we take the minimum of 5 and the length of the bytes
                        let prefix = &bytes[..5.min(bytes.len())];
                        let prefix = unsafe { OsStr::from_encoded_bytes_unchecked(prefix) };

                        // $Env:LOCALAPPDATA
                        if prefix.eq_ignore_ascii_case("$Env:") {
                            Some(&bytes[5..])
                        } else if bytes[0] == b'$' {
                            // $LOCALAPPDATA
                            Some(&bytes[1..])
                        } else {
                            // not a variable
                            None
                        }
                    };

                    // if component is a variable, get the value from the environment
                    if let Some(var) = var {
                        let var = unsafe { OsStr::from_encoded_bytes_unchecked(var) };
                        if let Ok(value) = std::env::var(var) {
                            out.push(value);
                            continue;
                        }
                    }

                    // if not a variable, or a value couldn't be obtained from environemnt
                    // then push the component as is
                    out.push(c);
                }

                // other components are pushed as is
                _ => out.push(c),
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn resolves_env_vars() {
        // helper functions
        fn path<P: AsRef<Path>>(p: P) -> PathBuf {
            // Ensure that the path is using the correct path separator for the OS.
            p.as_ref().components().collect::<PathBuf>()
        }

        fn expand<P: AsRef<Path>>(p: P) -> PathBuf {
            p.expand_vars()
        }

        // Set a variable for testing
        std::env::set_var("VAR", "VALUE");

        // %VAR% format
        assert_eq!(expand("/path/%VAR%/to/dir"), path("/path/VALUE/to/dir"));
        // $env:VAR format
        assert_eq!(expand("/path/$env:VAR/to/dir"), path("/path/VALUE/to/dir"));
        // $VAR format
        assert_eq!(expand("/path/$VAR/to/dir"), path("/path/VALUE/to/dir"));

        // non-existent variable
        assert_eq!(expand("/path/%ASD%/to/d"), path("/path/%ASD%/to/d"));
        assert_eq!(expand("/path/$env:ASD/to/d"), path("/path/$env:ASD/to/d"));
        assert_eq!(expand("/path/$ASD/to/d"), path("/path/$ASD/to/d"));

        // Set a $env:USERPROFILE variable for testing
        #[cfg(windows)]
        std::env::set_var("USERPROFILE", "C:\\Users\\user");

        // Set a $HOME variable for testing
        #[cfg(not(windows))]
        std::env::set_var("HOME", "C:\\Users\\user");

        // ~ and $HOME should be replaced with $Env:USERPROFILE
        assert_eq!(expand("~"), path("C:\\Users\\user"));
        assert_eq!(expand("$HOME"), path("C:\\Users\\user"));
    }
}
