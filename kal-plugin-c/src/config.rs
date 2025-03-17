/// Configuration API for C interop
///
/// This module provides functions for accessing configuration values from C code.
/// It defines an opaque type `Config` and functions to get boolean, string, and integer values.
///
/// ## C API
///
/// Three main functions are provided for the C API:
/// - `config_get_bool`: Gets a boolean value from the configuration
/// - `config_get_str`: Gets a string value from the configuration
/// - `config_get_int`: Gets an integer value from the configuration
///
/// ## Rust API
///
/// The `rust` submodule provides Rust-friendly wrappers around the C API functions:
/// - `config_get_bool`: Gets a boolean value from the configuration
/// - `config_get_str`: Gets a string value from the configuration
/// - `config_get_int`: Gets an integer value from the configuration
///
/// ## Implementation Note
///
/// The C API functions use the `to_rust!` macro to convert from C types to Rust types before
/// accessing configuration values.
use std::ffi::*;

/// The configuration opaque type
pub type Config = c_void;

/// Convert the C configuration types to Rust types
macro_rules! to_rust {
    ($config:ident, $plugin:ident, $key:ident) => {
        let config = $config as *const kal_config::Config;
        let $config = unsafe { &*config };

        let plugin = unsafe { CStr::from_ptr($plugin) };
        let $plugin = plugin.to_str().unwrap_or("");

        let key = unsafe { CStr::from_ptr($key) };
        let $key = key.to_str().unwrap_or("");
    };
}

/// Get a boolean value from the configuration
#[no_mangle]
pub extern "C" fn config_get_bool(
    config: *const Config,
    plugin: *const c_char,
    key: *const c_char,
) -> u8 {
    to_rust!(config, plugin, key);

    config
        .plugins
        .get(plugin)
        .and_then(|c| c.inner.as_ref())
        .and_then(|c| c.get(key))
        .and_then(|v| v.as_bool())
        .unwrap_or_default() as u8
}

/// Get a string value from the configuration
#[no_mangle]
pub extern "C" fn config_get_str(
    config: *const Config,
    plugin: *const c_char,
    key: *const c_char,
) -> *const c_char {
    to_rust!(config, plugin, key);

    let value = config
        .plugins
        .get(plugin)
        .and_then(|c| c.inner.as_ref())
        .and_then(|c| c.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    CString::new(value).unwrap().into_raw()
}

/// Get an integer value from the configuration
#[no_mangle]
pub extern "C" fn config_get_int(
    config: *const Config,
    plugin: *const c_char,
    key: *const c_char,
) -> i64 {
    to_rust!(config, plugin, key);

    config
        .plugins
        .get(plugin)
        .and_then(|c| c.inner.as_ref())
        .and_then(|c| c.get(key))
        .and_then(|v| v.as_integer())
        .unwrap_or_default()
}

/// Rust-friendly wrappers for the C API functions
///
/// This module provides idiomatic Rust interfaces to the configuration API functions,
/// handling C string conversions and proper type safety.
pub mod rust {
    /// Get a boolean value from the configuration
    pub fn config_get_bool(config: *const crate::Config, plugin: &str, key: &str) -> bool {
        let plugin = std::ffi::CString::new(plugin).unwrap();
        let key = std::ffi::CString::new(key).unwrap();
        let bool = super::config_get_bool(config, plugin.as_ptr(), key.as_ptr());
        bool != 0
    }

    /// Get a string value from the configuration
    pub fn config_get_str(config: *const crate::Config, plugin: &str, key: &str) -> String {
        let plugin = std::ffi::CString::new(plugin).unwrap();
        let key = std::ffi::CString::new(key).unwrap();
        let c_str = super::config_get_str(config, plugin.as_ptr(), key.as_ptr());
        unsafe {
            std::ffi::CString::from_raw(c_str as _)
                .into_string()
                .unwrap()
        }
    }

    /// Get an integer value from the configuration
    pub fn config_get_int(config: *const crate::Config, plugin: &str, key: &str) -> i64 {
        let plugin = std::ffi::CString::new(plugin).unwrap();
        let key = std::ffi::CString::new(key).unwrap();
        super::config_get_int(config, plugin.as_ptr(), key.as_ptr())
    }
}
