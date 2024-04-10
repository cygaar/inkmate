use alloc::string::{String, ToString};
use core::marker::PhantomData;

use stylus_sdk::{
    abi,
    alloy_primitives::{Address, B256, U256},
    alloy_sol_types::{sol, SolType},
    block,
    crypto::keccak,
    evm, msg,
    prelude::*,
};

pub trait ERC20Params {
    const NAME: &'static str;
    const SYMBOL: &'static str;
    const DECIMALS: u8;
}

type SolStructHash = sol! { tuple(bytes32, address, address, uint256, uint256, uint256) };
type SolSignedHash = sol! { tuple(string, bytes32, bytes32) };

sol_storage! {
    /// ERC20 implements all ERC-20 methods.
    pub struct ERC20<T> {
        /// Maps users to balances
        mapping(address => uint256) balances;
        /// Maps users to a mapping of each spender's allowance
        mapping(address => mapping(address => uint256)) allowances;
        /// The total supply of the token
        uint256 total_supply;
        /// Nonces used for EIP2612
        mapping(address => uint256) nonces;
        /// Used to allow [`ERC20Params`]
        PhantomData<T> phantom;
    }
}

// Declare events and Solidity error types
sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    error InsufficientBalance(address from, uint256 have, uint256 want);
    error InsufficientAllowance(address owner, address spender, uint256 have, uint256 want);
    error PermitExpired();
}

#[derive(SolidityError)]
pub enum ERC20Error {
    InsufficientBalance(InsufficientBalance),
    InsufficientAllowance(InsufficientAllowance),
    PermitExpired(PermitExpired),
}

// Internal functions
impl<T: ERC20Params> ERC20<T> {
    pub fn _transfer(&mut self, from: Address, to: Address, value: U256) -> Result<(), ERC20Error> {
        let mut sender_balance = self.balances.setter(from);
        let old_sender_balance = sender_balance.get();
        if old_sender_balance < value {
            return Err(ERC20Error::InsufficientBalance(InsufficientBalance {
                from,
                have: old_sender_balance,
                want: value,
            }));
        }
        sender_balance.set(old_sender_balance - value);
        let mut to_balance = self.balances.setter(to);
        let new_to_balance = to_balance.get() + value;
        to_balance.set(new_to_balance);
        evm::log(Transfer { from, to, value });
        Ok(())
    }

    pub fn _mint(&mut self, address: Address, value: U256) {
        let mut balance = self.balances.setter(address);
        let new_balance = balance.get() + value;
        balance.set(new_balance);
        self.total_supply.set(self.total_supply.get() + value);
        evm::log(Transfer {
            from: Address::ZERO,
            to: address,
            value,
        });
    }

    pub fn _burn(&mut self, address: Address, value: U256) -> Result<(), ERC20Error> {
        let mut balance = self.balances.setter(address);
        let old_balance = balance.get();
        if old_balance < value {
            return Err(ERC20Error::InsufficientBalance(InsufficientBalance {
                from: address,
                have: old_balance,
                want: value,
            }));
        }
        balance.set(old_balance - value);
        self.total_supply.set(self.total_supply.get() - value);
        evm::log(Transfer {
            from: address,
            to: Address::ZERO,
            value,
        });
        Ok(())
    }

    pub fn _domain_separator(&mut self) -> B256 {

    }
}

// These methods are external to other contracts
#[external]
impl<T: ERC20Params> ERC20<T> {
    pub fn name() -> String {
        T::NAME.into()
    }

    pub fn symbol() -> String {
        T::SYMBOL.into()
    }

    pub fn decimals() -> u8 {
        T::DECIMALS
    }

    pub fn total_supply(&self) -> U256 {
        self.total_supply.get()
    }

    pub fn balance_of(&self, address: Address) -> U256 {
        self.balances.get(address)
    }

    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.allowances.getter(owner).get(spender)
    }

    pub fn transfer(&mut self, to: Address, value: U256) -> Result<bool, ERC20Error> {
        self._transfer(msg::sender(), to, value)?;
        Ok(true)
    }

    pub fn approve(&mut self, spender: Address, value: U256) -> bool {
        self.allowances.setter(msg::sender()).insert(spender, value);
        evm::log(Approval {
            owner: msg::sender(),
            spender,
            value,
        });
        true
    }

    pub fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        value: U256,
    ) -> Result<bool, ERC20Error> {
        let mut sender_allowances = self.allowances.setter(from);
        let mut allowance = sender_allowances.setter(msg::sender());
        let old_allowance = allowance.get();
        if old_allowance < value {
            return Err(ERC20Error::InsufficientAllowance(InsufficientAllowance {
                owner: from,
                spender: msg::sender(),
                have: old_allowance,
                want: value,
            }));
        }
        allowance.set(old_allowance - value);
        self._transfer(from, to, value)?;
        Ok(true)
    }

    pub fn DOMAIN_SEPARATOR( &mut self) -> Result<B256, ERC20Error> {
        Ok(self._domain_separator())
    }

    pub fn permit(
        &mut self,
        owner: Address,
        spender: Address,
        value: U256,
        deadline: U256,
        v: u8,
        r: B256,
        s: B256,
    ) -> Result<(), ERC20Error> {
        if U256::from(block::timestamp()) > deadline {
            return Err(ERC20Error::PermitExpired(PermitExpired {}));
        }

        let nonce = self.nonces.get(owner);
        self.nonces.setter(owner).set(nonce + U256::from(1));

        let domain_separator = self._domain_separator().as_slice().try_into().expect("Should be 32 bytes");

        let permit_typehash = keccak(
            "Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)",
        );
        let permit_typehash_array = permit_typehash
            .as_slice()
            .try_into()
            .expect("Slice must be exactly 32 bytes");
        let struct_hash = SolStructHash::encode(&(
            permit_typehash_array,
            owner,
            spender,
            value,
            nonce,
            deadline,
        ));
        let struct_hash_array = struct_hash.as_slice().try_into().expect("Slice must be exactly 32 bytes");
        let signed_hash = SolSignedHash::encode(&("\x19\x01".to_string(), domain_separator, struct_hash_array));

        Ok(())
    }
}
