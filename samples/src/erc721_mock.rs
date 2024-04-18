//! An example ERC721 contract
extern crate alloc;

use crate::inkmate::tokens::erc721::{ERC721Params, ERC721};
use alloc::{format, string::String, vec::Vec};
use stylus_sdk::{alloy_primitives::U256, msg, prelude::*};

pub struct ERC721MockParams;

/// Immutable definitions
impl ERC721Params for ERC721MockParams {
    const NAME: &'static str = "ERC721 Stylus Example";
    const SYMBOL: &'static str = "MOCK";

    fn token_uri(token_id: U256) -> String {
        format!(
            "ipfs://QmZcH4YvBVVRJtdn4RdbaqgspFU8gH6P9vomDpBVpAL3u4/{}",
            token_id
        )
    }
}

sol_storage! {
    #[entrypoint] // Makes ERC721Mock the entrypoint
    pub struct ERC721Mock {
        #[borrow]
        ERC721<ERC721MockParams> erc721;
        uint256 total_supply;
    }
}

#[external]
#[inherit(ERC721<ERC721MockParams>)]
impl ERC721Mock {
    pub fn total_supply(&self) -> U256 {
        self.total_supply.get()
    }

    pub fn mint_loop(&mut self, qty: U256) -> Result<(), Vec<u8>> {
        let supply = self.total_supply.get();
        let supply: u32 = supply.try_into().unwrap();
        let qty: u32 = qty.try_into().unwrap();

        for i in 0..qty.try_into().unwrap() {
            self.erc721.mint(msg::sender(), U256::from(supply + i))?;
        }
        self.total_supply.set(U256::from(supply + qty));
        Ok(())
    }

    pub fn burn(&mut self, token_id: U256) -> Result<(), Vec<u8>> {
        self.erc721.burn(token_id)?;
        let supply = self.total_supply.get();
        self.total_supply.set(supply - U256::from(1));
        Ok(())
    }
}
