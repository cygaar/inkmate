//! An example ERC20 contract
extern crate alloc;

use crate::inkmate::tokens::erc20::{ERC20Params, ERC20};
use alloc::vec::Vec;
use stylus_sdk::{alloy_primitives::U256, msg, prelude::*};

struct ERC20MockParams;

/// Immutable definitions
impl ERC20Params for ERC20MockParams {
    const NAME: &'static str = "ERC20 Stylus Example";
    const SYMBOL: &'static str = "MOCK";
    const DECIMALS: u8 = 18;
}

sol_storage! {
    #[entrypoint] // Makes ERC20Mock the entrypoint
    struct ERC20Mock {
        #[borrow]
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
