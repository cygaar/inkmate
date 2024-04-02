//! A test contract inheriting from the ERC20 base contract, and exposing some of its internal helper methods

use crate::contracts::erc20::{Erc20, Erc20Params};
use alloc::vec::Vec;
use stylus_sdk::{alloy_primitives::U256, msg, prelude::*};

struct ERC20MockParams;

/// Immutable definitions
impl Erc20Params for ERC20MockParams {
    const NAME: &'static str = "ERC20 Example";
    const SYMBOL: &'static str = "MOCK";
    const DECIMALS: u8 = 18;
}

// The contract
sol_storage! {
    #[entrypoint] // Makes ERC20Mock the entrypoint
    struct ERC20Mock {
        #[borrow] // Allows erc20 to access ERC20Mock's storage and make calls
        Erc20<ERC20MockParams> erc20;
    }
}

#[external]
#[inherit(Erc20<ERC20MockParams>)]
impl ERC20Mock {
    pub fn mint(&mut self, qty: U256) -> Result<(), Vec<u8>> {
        self.erc20.mint(msg::sender(), qty);
        Ok(())
    }

    pub fn burn(&mut self, qty: U256) -> Result<(), Vec<u8>> {
        self.erc20.burn(msg::sender(), qty)?;
        Ok(())
    }
}
