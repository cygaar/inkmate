//! ERC20 base contract with EIP2612 (permit) support.
//! Doc comments are forked from: https://github.com/Vectorized/solady/blob/main/src/tokens/ERC20.sol

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

use crate::inkmate_common::crypto::ecrecover::EcRecoverTrait;
use crate::utils::ecrecover::PrecompileEcRecover;

pub trait ERC20Params {
    const NAME: &'static str;
    const SYMBOL: &'static str;
    const DECIMALS: u8;
}

sol_storage! {
    pub struct ERC20<T> {
        uint256 total_supply;
        mapping(address => uint256) balances;
        mapping(address => mapping(address => uint256)) allowances;
        mapping(address => uint256) nonces;
        PhantomData<T> phantom;
    }
}

// Define events and errors in the contract
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

// keccak256("1")
const VERSION_HASH: B256 =
    fixed_bytes!("c89efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6");

// keccack256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
const EIP_712_DOMAIN_HASH: B256 =
    fixed_bytes!("8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f");

// keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)")
const PERMIT_TYPEHASH: B256 =
    fixed_bytes!("6e71edae12b1b97f4d1f60370fef10105fa2faae0126114a169c64845d6126c9");

// Internal functions
impl<T: ERC20Params> ERC20<T> {
    /// Moves `amount` of tokens from `from` to `to`.
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

    /// Mints `amount` tokens to `to`, increasing the total supply.
    ///
    /// Emits a {Transfer} event.
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

    /// Burns `amount` tokens from `from`, reducing the total supply.
    ///
    /// Emits a {Transfer} event.
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

    /// Computes the domain separator for the current contract and chain
    pub fn _compute_domain_separator(&self) -> B256 {
        keccak(
            <sol! { (bytes32, bytes32, bytes32, uint256, address) }>::encode(&(
                EIP_712_DOMAIN_HASH.0,
                keccak(T::NAME.as_bytes()).0,
                VERSION_HASH.0,
                U256::from(block::chainid()),
                contract::address(),
            )),
        )
    }
}

// External functions
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

    /// Returns the amount of tokens in existence.
    pub fn total_supply(&self) -> U256 {
        self.total_supply.get()
    }

    /// Returns the amount of tokens owned by `owner`.
    pub fn balance_of(&self, address: Address) -> U256 {
        self.balances.get(address)
    }

    /// Returns the amount of tokens that `spender` can spend on behalf of `owner`.
    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.allowances.getter(owner).get(spender)
    }

    /// Transfer `amount` tokens from the caller to `to`.
    ///
    /// Requirements:
    /// - `from` must at least have `amount`.
    ///
    /// Emits a {Transfer} event.
    pub fn transfer(&mut self, to: Address, value: U256) -> Result<bool, ERC20Error> {
        self._transfer(msg::sender(), to, value)?;
        Ok(true)
    }

    /// Sets `amount` as the allowance of `spender` over the caller's tokens.
    ///
    /// Emits a {Approval} event.
    pub fn approve(&mut self, spender: Address, value: U256) -> bool {
        self.allowances.setter(msg::sender()).insert(spender, value);
        evm::log(Approval {
            owner: msg::sender(),
            spender,
            value,
        });
        true
    }

    /// Transfers `amount` tokens from `from` to `to`.
    ///
    /// Requirements:
    /// - `from` must at least have `amount`.
    /// - The caller must have at least `amount` of allowance to transfer the tokens of `from`.
    ///
    /// Emits a {Transfer} event.
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

    /// @dev Returns the EIP-712 domain separator for the EIP-2612 permit.
    #[selector(name = "DOMAIN_SEPARATOR")]
    pub fn domain_separator(&self) -> B256 {
        self._compute_domain_separator()
    }

    /// @dev Sets `value` as the allowance of `spender` over the tokens of `owner`,
    /// authorized by a signed approval by `owner`.
    ///
    /// Emits a {Approval} event.
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

        let struct_hash = keccak(
            <sol! { (bytes32, address, address, uint256, uint256, uint256) }>::encode(&(
                PERMIT_TYPEHASH.0,
                owner,
                spender,
                value,
                nonce,
                deadline,
            )),
        );

        let signed_hash = keccak(<sol! { (string, bytes32, bytes32) }>::encode_packed(&(
            "\x19\x01".to_string(),
            self._compute_domain_separator().0,
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

        evm::log(Approval {
            owner,
            spender,
            value,
        });

        Ok(())
    }
}
