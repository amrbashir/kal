#![allow(unused)]

use std::ffi::*;

use ::safer_ffi::prelude::*;

pub mod action;
pub mod config;
pub mod icon;
pub mod result_item;

pub use action::*;
pub use config::*;
pub use icon::*;
pub use result_item::*;

// The following function is only necessary for the header generation.
#[cfg(feature = "headers")]
pub fn generate_headers() -> ::std::io::Result<()> {
    ::safer_ffi::headers::builder()
        .to_file("kal-plugin.h")?
        .generate()
}
