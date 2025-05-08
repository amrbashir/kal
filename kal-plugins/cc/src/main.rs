use std::ffi::*;

use kal_plugin::{CResultItem, ResultItem};

fn main() {
    let config = kal_config::Config::load().unwrap();

    unsafe {
        let lib = libloading::Library::new(r"D:\.cargo-target\debug\cc.dll").unwrap();
        let new = lib
            .get::<unsafe extern "C" fn() -> *const c_void>(b"new")
            .unwrap();
        let destroy = lib
            .get::<unsafe extern "C" fn(*const c_void)>(b"destroy")
            .unwrap();
        let name = lib
            .get::<unsafe extern "C" fn(*const c_void) -> *const c_char>(b"name")
            .unwrap();
        let reload = lib
            .get::<unsafe extern "C" fn(*const c_void, *const c_void)>(b"reload")
            .unwrap();
        let query_direct = lib
            .get::<unsafe extern "C" fn(
                *const c_void,
                *const c_char,
                extern "C" fn(*const c_char, *const c_char) -> u16,
                *mut usize,
            ) -> *const c_void>(b"query_direct")
            .unwrap();

        let plugin = new();

        dbg!(CString::from_raw(name(plugin) as _)
            .to_str()
            .unwrap()
            .to_string());

        reload(plugin, &config as *const _ as *const _);

        let s = "Some query";
        let matcher = Box::new(|query: &str, needle: &str| {
            dbg!(query);
            dbg!(needle);
        });

        let mut len = 0_usize;

        destroy(plugin);
    }
}
