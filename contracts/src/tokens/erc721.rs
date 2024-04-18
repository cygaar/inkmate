//! ERC721 base contract.
//! The logic was based off of: https://github.com/OffchainLabs/stylus-workshop-nft/blob/main/src/erc712.rs
//! Doc comments are forked from: https://github.com/Vectorized/solady/blob/main/src/tokens/ERC721.sol

use alloc::{string::String, vec, vec::Vec};
use core::{borrow::BorrowMut, marker::PhantomData};
use stylus_sdk::{
    abi::Bytes,
    alloy_primitives::{b256, Address, U256},
    alloy_sol_types::{sol, SolType},
    evm, msg,
    prelude::*,
};

pub trait ERC721Params {
    const NAME: &'static str;
    const SYMBOL: &'static str;
    fn token_uri(token_id: U256) -> String;
}

sol_storage! {
    /// ERC721 implements all ERC-721 methods
    pub struct ERC721<T: ERC721Params> {
        mapping(uint256 => address) owners;
        mapping(uint256 => address) approved;
        mapping(address => uint256) balance;
        mapping(address => mapping(address => bool)) approved_for_all;
        PhantomData<T> phantom;
    }
}

// Declare events and Solidity error types
sol! {
    /// Emitted when token `id` is transferred from `from` to `to`.
    event Transfer(address indexed from, address indexed to, uint256 indexed token_id);
    /// Emitted when `owner` enables `account` to manage the `id` token.
    event Approval(address indexed owner, address indexed approved, uint256 indexed token_id);
    /// Emitted when `owner` enables or disables `operator` to manage all of their tokens.
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);

    error AlreadyMinted();
    error InvalidTokenId(uint256 token_id);
    error NotOwner(address from, uint256 token_id, address real_owner);
    error NotApproved(uint256 token_id, address owner, address spender);
    error TransferToZero(uint256 token_id);
    error ReceiverRefused(address receiver, uint256 token_id);
}

#[derive(SolidityError)]
pub enum ERC721Error {
    AlreadyMinted(AlreadyMinted),
    InvalidTokenId(InvalidTokenId),
    NotOwner(NotOwner),
    NotApproved(NotApproved),
    TransferToZero(TransferToZero),
    ReceiverRefused(ReceiverRefused),
    ExternalCall(stylus_sdk::call::Error),
}

// These methods aren't external, but are helpers used by external methods.
// Methods marked as "pub" here are usable outside of the erc721 module (i.e. they're callable from main.rs).
impl<T: ERC721Params> ERC721<T> {
    /// Requires that msg::sender() is authorized to spend a given token
    fn require_authorized_to_spend(
        &self,
        from: Address,
        token_id: U256,
    ) -> Result<(), ERC721Error> {
        let owner = self.owner_of(token_id)?;
        if from != owner {
            return Err(ERC721Error::NotOwner(NotOwner {
                from,
                token_id,
                real_owner: owner,
            }));
        }

        if msg::sender() == owner {
            return Ok(());
        }
        if self.approved_for_all.getter(owner).get(msg::sender()) {
            return Ok(());
        }
        if msg::sender() == self.approved.get(token_id) {
            return Ok(());
        }
        Err(ERC721Error::NotApproved(NotApproved {
            owner,
            spender: msg::sender(),
            token_id,
        }))
    }

    /// Transfers `token_id` from `from` to `to`.
    /// This function does check that `from` is the owner of the token, but it does not check
    /// that `to` is not the zero address, as this function is usable for burning.
    pub fn transfer(
        &mut self,
        token_id: U256,
        from: Address,
        to: Address,
    ) -> Result<(), ERC721Error> {
        let mut owner = self.owners.setter(token_id);
        let previous_owner = owner.get();
        if previous_owner != from {
            return Err(ERC721Error::NotOwner(NotOwner {
                from,
                token_id,
                real_owner: previous_owner,
            }));
        }
        owner.set(to);

        // right now working with storage can be verbose, but this will change upcoming version of the Stylus SDK
        let mut from_balance = self.balance.setter(from);
        let balance = from_balance.get() - U256::from(1);
        from_balance.set(balance);

        let mut to_balance = self.balance.setter(to);
        let balance = to_balance.get() + U256::from(1);
        to_balance.set(balance);

        self.approved.delete(token_id);
        evm::log(Transfer { from, to, token_id });
        Ok(())
    }

    fn call_receiver<S: TopLevelStorage>(
        storage: &mut S,
        token_id: U256,
        from: Address,
        to: Address,
        data: Vec<u8>,
    ) -> Result<(), ERC721Error> {
        // TODO: wait for Stylus SDK to fix the has_code property checker
        let hash = to.codehash();
        if !hash.is_zero()
            || hash == b256!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
        {
            let receiver = IERC721TokenReceiver::new(to);
            let received = receiver
                .on_erc_721_received(storage, msg::sender(), from, token_id, data)
                .map_err(|_| {
                    ERC721Error::ReceiverRefused(ReceiverRefused {
                        receiver: receiver.address,
                        token_id,
                    })
                })?
                .0;

            if u32::from_be_bytes(received) != ERC721_TOKEN_RECEIVER_ID {
                return Err(ERC721Error::ReceiverRefused(ReceiverRefused {
                    receiver: receiver.address,
                    token_id,
                }));
            }
        }
        Ok(())
    }

    pub fn safe_transfer<S: TopLevelStorage + BorrowMut<Self>>(
        storage: &mut S,
        token_id: U256,
        from: Address,
        to: Address,
        data: Vec<u8>,
    ) -> Result<(), ERC721Error> {
        storage.borrow_mut().transfer_from(from, to, token_id)?;
        Self::call_receiver(storage, token_id, from, to, data)
    }

    pub fn mint(&mut self, to: Address, token_id: U256) -> Result<(), ERC721Error> {
        if to.is_zero() {
            return Err(ERC721Error::TransferToZero(TransferToZero { token_id }));
        }
        let mut owner = self.owners.setter(token_id);
        if !owner.is_zero() {
            return Err(ERC721Error::AlreadyMinted(AlreadyMinted {}));
        }
        owner.set(to);

        let mut to_balance = self.balance.setter(to);
        let balance = to_balance.get() + U256::from(1);
        to_balance.set(balance);

        evm::log(Transfer {
            from: Address::default(),
            to,
            token_id,
        });
        Ok(())
    }

    pub fn safe_mint<S: TopLevelStorage + BorrowMut<Self>>(
        storage: &mut S,
        to: Address,
        token_id: U256,
        data: Vec<u8>,
    ) -> Result<(), ERC721Error> {
        storage.borrow_mut().mint(to, token_id)?;
        Self::call_receiver(storage, token_id, Address::default(), to, data)?;
        Ok(())
    }

    pub fn burn(&mut self, token_id: U256) -> Result<(), ERC721Error> {
        let mut owner_setter = self.owners.setter(token_id);
        if owner_setter.is_zero() {
            return Err(ERC721Error::InvalidTokenId(InvalidTokenId { token_id }));
        }
        let owner = owner_setter.get();

        if msg::sender() != owner
            && !self.approved_for_all.getter(owner).get(msg::sender())
            && msg::sender() != self.approved.get(token_id)
        {
            return Err(ERC721Error::NotApproved(NotApproved {
                owner,
                spender: msg::sender(),
                token_id,
            }));
        }

        let mut owner_balance = self.balance.setter(owner);
        let balance = owner_balance.get() - U256::from(1);
        owner_balance.set(balance);

        owner_setter.set(Address::default());
        self.approved.delete(token_id);

        evm::log(Transfer {
            from: owner,
            to: Address::default(),
            token_id,
        });
        Ok(())
    }
}

sol_interface! {
    /// Allows calls to the `onERC721Received` method of other contracts implementing `IERC721TokenReceiver`.
    interface IERC721TokenReceiver {
        function onERC721Received(address operator, address from, uint256 token_id, bytes data) external returns(bytes4);
    }
}

/// Selector for `onERC721Received`, which is returned by contracts implementing `IERC721TokenReceiver`.
const ERC721_TOKEN_RECEIVER_ID: u32 = 0x150b7a02;

// these methods are external to other contracts
#[external]
impl<T: ERC721Params> ERC721<T> {
    /// Immutable NFT name.
    pub fn name() -> String {
        T::NAME.into()
    }

    /// Immutable NFT symbol.
    pub fn symbol() -> String {
        T::SYMBOL.into()
    }

    /// The NFT's Uniform Resource Identifier.
    #[selector(name = "tokenURI")]
    pub fn token_uri(&self, token_id: U256) -> Result<String, ERC721Error> {
        self.owner_of(token_id)?; // require NFT exist
        Ok(T::token_uri(token_id))
    }

    /// Wether the NFT supports a given standard.
    pub fn supports_interface(interface: [u8; 4]) -> bool {
        if interface == [0xff; 4] {
            // special cased in the ERC165 standard
            return false;
        }

        const IERC165: u32 = 0x01ffc9a7;
        const IERC721: u32 = 0x80ac58cd;
        const IERC721METADATA: u32 = 0x5b5e139f;

        matches!(
            u32::from_be_bytes(interface),
            IERC165 | IERC721 | IERC721METADATA
        )
    }

    /// Gets the number of NFTs owned by an account.
    pub fn balance_of(&self, owner: Address) -> U256 {
        U256::from(self.balance.get(owner))
    }

    /// Gets the owner of the NFT, if it exists.
    pub fn owner_of(&self, token_id: U256) -> Result<Address, ERC721Error> {
        let owner = self.owners.get(token_id);
        if owner.is_zero() {
            return Err(ERC721Error::InvalidTokenId(InvalidTokenId { token_id }));
        }
        Ok(owner)
    }

    /// Transfers an NFT, but only after checking the `to` address can receive the NFT.
    pub fn safe_transfer_from<S: TopLevelStorage + BorrowMut<Self>>(
        storage: &mut S,
        from: Address,
        to: Address,
        token_id: U256,
    ) -> Result<(), ERC721Error> {
        Self::safe_transfer_from_with_data(storage, from, to, token_id, Bytes(vec![]))
    }

    /// Equivalent to [`safe_transfer_from`], but with additional data for the receiver.
    ///
    /// Note: because Rust doesn't allow multiple methods with the same name,
    /// we use the `#[selector]` macro attribute to simulate solidity overloading.
    #[selector(name = "safeTransferFrom")]
    pub fn safe_transfer_from_with_data<S: TopLevelStorage + BorrowMut<Self>>(
        storage: &mut S,
        from: Address,
        to: Address,
        token_id: U256,
        data: Bytes,
    ) -> Result<(), ERC721Error> {
        if to.is_zero() {
            return Err(ERC721Error::TransferToZero(TransferToZero { token_id }));
        }
        storage
            .borrow_mut()
            .require_authorized_to_spend(from, token_id)?;

        Self::safe_transfer(storage, token_id, from, to, data.0)
    }

    /// Transfers the NFT.
    pub fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
    ) -> Result<(), ERC721Error> {
        if to.is_zero() {
            return Err(ERC721Error::TransferToZero(TransferToZero { token_id }));
        }
        self.require_authorized_to_spend(from, token_id)?;
        self.transfer(token_id, from, to)?;
        Ok(())
    }

    /// Grants an account the ability to manage the sender's NFT.
    pub fn approve(&mut self, approved: Address, token_id: U256) -> Result<(), ERC721Error> {
        let owner = self.owner_of(token_id)?;

        // require authorization
        if msg::sender() != owner && !self.approved_for_all.getter(owner).get(msg::sender()) {
            return Err(ERC721Error::NotApproved(NotApproved {
                owner,
                spender: msg::sender(),
                token_id,
            }));
        }
        self.approved.insert(token_id, approved);

        evm::log(Approval {
            approved,
            owner,
            token_id,
        });
        Ok(())
    }

    /// Grants an account the ability to manage all of the sender's NFTs.
    pub fn set_approval_for_all(&mut self, operator: Address, approved: bool) {
        let owner = msg::sender();
        self.approved_for_all
            .setter(owner)
            .insert(operator, approved);

        evm::log(ApprovalForAll {
            owner,
            operator,
            approved,
        });
    }

    /// Gets the account managing an NFT, or zero if unmanaged.
    pub fn get_approved(&mut self, token_id: U256) -> Address {
        self.approved.get(token_id)
    }

    /// Determines if an account has been authorized to managing all of a user's NFTs.
    pub fn is_approved_for_all(&mut self, owner: Address, operator: Address) -> bool {
        self.approved_for_all.getter(owner).get(operator)
    }
}
