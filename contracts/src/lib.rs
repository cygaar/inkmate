#![cfg_attr(not(feature = "export-abi"), no_main, no_std)]
extern crate alloc;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: mini_alloc::MiniAlloc = mini_alloc::MiniAlloc::INIT;

extern crate common;

pub mod erc20;
mod examples;
mod utils;
