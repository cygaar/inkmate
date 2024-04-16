//! This file is used to generate an ABI since there is no executable to generate the abi in the base library.
//! To generate the abi, you can run: `cargo run --example erc20_abi --features=export-abi,erc20`
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
