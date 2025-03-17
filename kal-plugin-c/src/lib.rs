#![allow(unused)]

use std::ffi::*;

pub mod action;
pub mod config;
pub mod icon;
pub mod result_item;

pub use action::*;
pub use config::*;
pub use icon::*;
pub use result_item::*;

/// The matcher function signature
pub type UnsafeMatcherFn = extern "C" fn(haystack: *const c_char, needle: *const c_char) -> u16;

#[macro_export]
macro_rules! define_plugin {
    ($plugin:ident) => {
        #[no_mangle]
        extern "C" fn new(config: *const $crate::Config) -> *mut $plugin {
            let plugin = $plugin::new(config);
            ::std::boxed::Box::into_raw(Box::new(plugin))
        }

        #[no_mangle]
        extern "C" fn destroy(this: *mut $plugin) {
            drop(unsafe { ::std::boxed::Box::from_raw(this) });
        }

        #[no_mangle]
        extern "C" fn name(this: *const $plugin) -> *const c_char {
            let name = (unsafe { &*this }).name();
            ::std::ffi::CString::new(name).unwrap().into_raw()
        }

        #[no_mangle]
        extern "C" fn default_plugin_config(this: *const $plugin) -> *const c_char {
            let config = (unsafe { &*this }).default_plugin_config();
            ::std::ffi::CString::new(config).unwrap().into_raw()
        }

        #[no_mangle]
        extern "C" fn reload(this: *mut $plugin, config: *const $crate::Config) {
            (unsafe { &mut *this }).reload(config);
        }

        #[no_mangle]
        extern "C" fn query(
            this: *mut $plugin,
            query: *const c_char,
            matcher_fn: *const $crate::UnsafeMatcherFn,
            len: *mut usize,
        ) -> *const $crate::CResultItem {
            let this = unsafe { &mut *this };

            let query = unsafe { ::std::ffi::CStr::from_ptr(query) };
            let query = query.to_str().unwrap_or("");

            let matcher_fn = unsafe { *matcher_fn };
            let matcher_fn = move |haystack: &str, needle: &str| -> u16 {
                let haystack_c = ::std::ffi::CString::new(haystack).unwrap();
                let needle_c = ::std::ffi::CString::new(needle).unwrap();
                matcher_fn(haystack_c.as_ptr(), needle_c.as_ptr())
            };

            let items = this.query(query, matcher_fn);

            unsafe { len.write(items.len()) };
            ::std::boxed::Box::into_raw(items.into_boxed_slice()) as *const CResultItem
        }

        #[no_mangle]
        extern "C" fn query_direct(
            this: *mut $plugin,
            query: *const c_char,
            matcher_fn: *const $crate::UnsafeMatcherFn,
            len: *mut usize,
        ) -> *const $crate::CResultItem {
            let this = unsafe { &mut *this };

            let query = unsafe { ::std::ffi::CStr::from_ptr(query) };
            let query = query.to_str().unwrap_or("");

            dbg!(3);
            let matcher_fn = unsafe { *matcher_fn };
            dbg!(4);
            let matcher_fn = move |haystack: &str, needle: &str| -> u16 {
                dbg!(5);
                let haystack_c = ::std::ffi::CString::new(haystack).unwrap();
                dbg!(6);
                let needle_c = ::std::ffi::CString::new(needle).unwrap();
                dbg!(7);
                matcher_fn(haystack_c.as_ptr(), needle_c.as_ptr())
            };
            dbg!(8);

            let items = this.query_direct(query, matcher_fn);
            dbg!(9);

            unsafe { len.write(items.len()) };
            dbg!(10);
            ::std::boxed::Box::into_raw(items.into_boxed_slice()) as *const CResultItem
        }
    };
}
