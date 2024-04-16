# 🖊️ inkmate

**Modern**, **opinionated**, and **gas optimized** building blocks for **smart contract development** in Rust for [Stylus](https://docs.arbitrum.io/stylus/stylus-gentle-introduction).

Stylus allows smart contracts to be written in any language that compiles down to web assembly. Stylus contracts are fully compatible with existing EVM smart contracts.
Stylus contracts can lead to large gas savings, especially when it comes to memory and compute. Stylus is only supported on Orbit chains based on **Arbitrum's Nitro tech stack**.

`inkmate` currently only supports **Rust** smart contracts, but further support for other languages may be added in the future.

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

## Safety

This is **experimental software** and is provided on an "as is" and "as available" basis.

We **do not give any warranties** and **will not be liable for any loss** incurred through any use of this codebase.

## Acknowledgements

These contracts were inspired by or directly modified from many sources, primarily:

- [solmate](https://github.com/transmissions11/solmate)
- [renegade](https://github.com/renegade-fi/renegade-contracts)
- [revm](https://github.com/bluealloy/revm)
