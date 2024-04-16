use alloc::string::{String, ToString};
use core::marker::PhantomData;
use stylus_sdk::{
    alloy_primitives::{fixed_bytes, Address, B256, U256},
    alloy_sol_types::{sol, SolType},
    block, contract,
    crypto::keccak,
    evm, msg,
    prelude::*,
};

use crate::common::crypto::ecrecover::EcRecoverTrait;
use crate::utils::ecrecover::PrecompileEcRecover;

pub trait ERC20Params {
    const NAME: &'static str;
    const SYMBOL: &'static str;
    const DECIMALS: u8;
}

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
    error InvalidPermit();
}

#[derive(SolidityError)]
pub enum ERC20Error {
    InsufficientBalance(InsufficientBalance),
    InsufficientAllowance(InsufficientAllowance),
    PermitExpired(PermitExpired),
    InvalidPermit(InvalidPermit),
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

    pub fn _domain_separator(&self) -> B256 {
        // keccack256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
        let eip712_domain_hash =
            fixed_bytes!("8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f");
        // keccak256("1")
        let version_hash =
            fixed_bytes!("c89efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6");
        let name_hash = keccak(T::NAME.as_bytes());

        keccak(
            <sol! { (bytes32, bytes32, bytes32, uint256, address) }>::encode(&(
                eip712_domain_hash.0,
                name_hash.0,
                version_hash.0,
                U256::from(block::chainid()),
                contract::address(),
            )),
        )
    }
}

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

    #[selector(name = "DOMAIN_SEPARATOR")]
    pub fn domain_separator(&self) -> B256 {
        self._domain_separator()
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

        // keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)")
        let permit_typehash =
            fixed_bytes!("6e71edae12b1b97f4d1f60370fef10105fa2faae0126114a169c64845d6126c9");

        let struct_hash = keccak(
            <sol! { (bytes32, address, address, uint256, uint256, uint256) }>::encode(&(
                permit_typehash.0,
                owner,
                spender,
                value,
                nonce,
                deadline,
            )),
        );

        let signed_hash = keccak(<sol! { (string, bytes32, bytes32) }>::encode_packed(&(
            "\x19\x01".to_string(),
            self._domain_separator().0,
            struct_hash.0,
        )));

        let recovered_address = Address::from_slice(
            &PrecompileEcRecover::ecrecover(&signed_hash.0, v, &r.0, &s.0)
                .map_err(|_| ERC20Error::InvalidPermit(InvalidPermit {}))?,
        );

        if recovered_address.is_zero() || recovered_address != owner {
            return Err(ERC20Error::InvalidPermit(InvalidPermit {}));
        }

        self.allowances
            .setter(recovered_address)
            .setter(spender)
            .set(value);

        Ok(())
    }
}
