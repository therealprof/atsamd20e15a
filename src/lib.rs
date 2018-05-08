#![no_std]
#![cfg_attr(feature = "rt", feature(global_asm))]
#![cfg_attr(feature = "rt", feature(used))]
#![feature(const_fn)]
#![allow(non_camel_case_types)]

extern crate bare_metal;
extern crate cortex_m;
extern crate cortex_m_rt;
extern crate vcell;

mod common;
pub mod snowflake;
mod svd;

pub use common::*;
pub use cortex_m_rt::*;
pub use svd::*;
