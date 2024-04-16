//! Building block contracts for Stylus

// Conditional compilation attributes for no_std compatibility and ABI export features
#![cfg_attr(not(feature = "export-abi"), no_main, no_std)]
extern crate alloc;

// Custom global allocator for the wasm32 target
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: mini_alloc::MiniAlloc = mini_alloc::MiniAlloc::INIT;

// Common utility contracts from the workspace
extern crate common;

// Conditional compilation of the ERC20 token module
#[cfg(any(feature = "erc20",))]
pub mod tokens;

// Module for example implementations and demos
mod examples;

// Utility functions and helpers used across the library
mod utils;
