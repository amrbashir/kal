use std::path::{Path, PathBuf};

#[cfg(windows)]
pub mod windows;

pub fn resolve_env_vars<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut out = PathBuf::new();

    for c in path.as_ref().components() {
        match c {
            std::path::Component::Normal(c) => {
                if let Some(c) = c.to_str() {
                    if c.starts_with('%') || c.ends_with('%') {
                        let var = c.strip_prefix('%').unwrap().strip_suffix('%').unwrap();
                        if let Ok(value) = std::env::var(var) {
                            out.push(value);
                            continue;
                        }
                    }
                }
                out.push(c);
            }
            _ => {
                out.push(c);
            }
        }
    }

    out
}
