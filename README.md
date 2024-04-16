# 🖊️ inkmate

**Modern**, **opinionated**, and **gas optimized** building blocks for **smart contract development** in Rust for [Stylus](https://docs.arbitrum.io/stylus/stylus-gentle-introduction).

Stylus allows smart contracts to be written in any language that compiles down to web assembly. Stylus contracts are fully compatible with existing EVM smart contracts.
Stylus contracts can lead to large gas savings, especially when it comes to memory and compute. Stylus is only supported on Orbit chains based on **Arbitrum's Nitro tech stack**.

`inkmate` currently only supports **Rust** smart contracts, but further support for other languages may be added in the future.

## Safety

This is **experimental software** and is provided on an "as is" and "as available" basis. **This code has not been audited!**

We **do not give any warranties** and **will not be liable for any loss** incurred through any use of this codebase.

## Contracts

```ml
tokens
├─ ERC20 — "Minimalist and gas efficient ERC20 + EIP-2612 implementation"
├─ ERC721 (coming soon) — "Minimalist and gas efficient ERC721 implementation"
├─ ERC721A (coming soon) — "Gas efficient ERC721 implementation with cheap minting costs"
utils
├─ ECRECOVER — "Library for calling ecrecover in Rust smart contracts"
```

## Installation

You'll first need to install the necessary tooling to support Rust Stylus contracts by following the official [installation guide](https://docs.arbitrum.io/stylus/stylus-quickstart).

Once you've done that, you can start a new Stylus project by running:
```bash
cargo stylus new <YOUR_PROJECT_NAME>
```

Install `inkmate` by running:
```bash
cargo add inkmate
```

If you want to only install certain features (ex. erc20), you can run:
```bash
cargo add inkmate --features "erc20"
```

Alternatively, you can add the following to your `Cargo.toml` file:
```toml
[dependencies]
inkmate = { version = "0.0.1", features = ["erc20"] }
```

Here's an example contract that uses `inkmate`
```rust
extern crate alloc;

use alloc::vec::Vec;
use inkmate::tokens::erc20::{ERC20Params, ERC20};
use stylus_sdk::{alloy_primitives::U256, msg, prelude::*};

struct ERC20MockParams;

/// Immutable definitions
impl ERC20Params for ERC20MockParams {
    const NAME: &'static str = "ERC20 Stylus Example";
    const SYMBOL: &'static str = "MOCK";
    const DECIMALS: u8 = 18;
}

// The contract
sol_storage! {
    #[entrypoint] // Makes ERC20Mock the entrypoint
    struct ERC20Mock {
        #[borrow] // Allows erc20 to access ERC20Mock's storage
        ERC20<ERC20MockParams> erc20;
    }
}

#[external]
#[inherit(ERC20<ERC20MockParams>)]
impl ERC20Mock {
    pub fn mint(&mut self, qty: U256) -> Result<(), Vec<u8>> {
        self.erc20._mint(msg::sender(), qty);
        Ok(())
    }

    pub fn burn(&mut self, qty: U256) -> Result<(), Vec<u8>> {
        self.erc20._burn(msg::sender(), qty)?;
        Ok(())
    }
}
```

## Contributing

This repo is setup as a single Rust workspace with two crates - `common` which contains common utility functions and `contracts` which contains the primary contract logic.

The `contracts` crate consists of multiple features to allow for conditional compilation and optional dependencies. This helps reduce binary sizes for Stylus contracts.

Because the `contracts` crate is feature gated, you cannot run `cargo stylus check` as you normally would.
To test the validity of our code (ex. erc20), there is a `mocks` folder which contains sample implementations used to create a sample binary.

To build the binary for your selected feature (ex. erc20), you can run:
```bash
cargo build --target wasm32-unknown-unknown --lib --release --features=erc20
```

Then to run check the validity of the contract you can run:
```bash
cargo stylus check --wasm-file-path target/wasm32-unknown-unknown/release/inkmate.wasm
```

Finally, you can deploy the contract to the Stylus testnet by running:
```bash
cargo stylus deploy -e https://stylus-testnet.arbitrum.io/rpc --private-key=<PRIVATE_KEY> --wasm-file-path target/wasm32-unknown-unknown/release/deps/inkmate.wasm
```

## Testing

Currently, only unit tests for specific pieces of logic are supported. A full set of integration tests will be added soon to test contract interaction logic.

To run unit tests, you can run:
```bash
cargo test -p inkmate-common
```

## Acknowledgements

These contracts were inspired by or directly modified from many sources, primarily:

- [solmate](https://github.com/transmissions11/solmate)
- [renegade](https://github.com/renegade-fi/renegade-contracts)
- [solady](https://github.com/Vectorized/solady)a
- [revm](https://github.com/bluealloy/revm)
- [stylus-sdk](https://github.com/OffchainLabs/stylus-sdk-rs)
