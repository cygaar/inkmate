//! ERC721a base contract.
//! The logic was based off of: https://github.com/chiru-labs/ERC721A
//! Doc comments are forked from: https://github.com/chiru-labs/ERC721A

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{borrow::BorrowMut, marker::PhantomData};
use stylus_sdk::{
    abi::Bytes,
    alloy_primitives::{b256, Address, U256, U64},
    alloy_sol_types::{sol, SolError},
    block, evm, msg,
    prelude::*,
};

pub trait ERC721Params {
    const NAME: &'static str;
    const SYMBOL: &'static str;
    fn token_uri(token_id: U256) -> String;
}

sol_storage! {
    pub struct TokenOwnership {
        address addr;
        uint64 start_timestamp;
        bool burned;
    }

    pub struct AddressData {
        uint64 balance;
        uint64 number_minted;
        uint64 number_burned;
        uint64 aux;
    }

    // TODO: figure out internal and private vars
    pub struct ERC721<T: ERC721Params> {
        uint256 _current_index;
        uint256 _burn_counter;
        mapping(uint256 => TokenOwnership) _ownerships;
        mapping(address => AddressData) _address_data;
        mapping(uint256 => address) _token_approvals;
        mapping(address => mapping(address => bool)) _operator_approvals;
        PhantomData<T> phantom;
    }
}

// Declare events and Solidity error types
sol! {
    event Transfer(address indexed from, address indexed to, uint256 indexed token_id);
    event Approval(address indexed owner, address indexed approved, uint256 indexed token_id);
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);

    error ApprovalCallerNotOwnerNorApproved();
    error ApprovalQueryForNonexistentToken();
    error ApprovalToCurrentOwner();
    error BalanceQueryForZeroAddress();
    error MintToZeroAddress();
    error MintZeroQuantity();
    error OwnerQueryForNonexistentToken();
    error ReentrancyGuard();
    error TransferCallerNotOwnerNorApproved();
    error TransferFromIncorrectOwner();
    error TransferToNonERC721ReceiverImplementer();
    error TransferToZeroAddress();
    error URIQueryForNonexistentToken();
}

/// Represents the ways methods may fail.
pub enum ERC721Error {
    ApprovalCallerNotOwnerNorApproved(ApprovalCallerNotOwnerNorApproved),
    ApprovalQueryForNonexistentToken(ApprovalQueryForNonexistentToken),
    ApprovalToCurrentOwner(ApprovalToCurrentOwner),
    BalanceQueryForZeroAddress(BalanceQueryForZeroAddress),
    MintToZeroAddress(MintToZeroAddress),
    MintZeroQuantity(MintZeroQuantity),
    OwnerQueryForNonexistentToken(OwnerQueryForNonexistentToken),
    ReentrancyGuard(ReentrancyGuard),
    TransferCallerNotOwnerNorApproved(TransferCallerNotOwnerNorApproved),
    TransferFromIncorrectOwner(TransferFromIncorrectOwner),
    TransferToNonERC721ReceiverImplementer(TransferToNonERC721ReceiverImplementer),
    TransferToZeroAddress(TransferToZeroAddress),
    URIQueryForNonexistentToken(URIQueryForNonexistentToken),
    ExternalCall(stylus_sdk::call::Error),
}

/// We will soon provide a `#[derive(SolidityError)]` to clean this up.
impl From<stylus_sdk::call::Error> for ERC721Error {
    fn from(err: stylus_sdk::call::Error) -> Self {
        Self::ExternalCall(err)
    }
}

/// We will soon provide a `#[derive(SolidityError)]` to clean this up.
impl From<ERC721Error> for Vec<u8> {
    fn from(val: ERC721Error) -> Self {
        match val {
            ERC721Error::ApprovalCallerNotOwnerNorApproved(err) => err.encode(),
            ERC721Error::ApprovalQueryForNonexistentToken(err) => err.encode(),
            ERC721Error::ApprovalToCurrentOwner(err) => err.encode(),
            ERC721Error::BalanceQueryForZeroAddress(err) => err.encode(),
            ERC721Error::MintToZeroAddress(err) => err.encode(),
            ERC721Error::MintZeroQuantity(err) => err.encode(),
            ERC721Error::OwnerQueryForNonexistentToken(err) => err.encode(),
            ERC721Error::ReentrancyGuard(err) => err.encode(),
            ERC721Error::TransferCallerNotOwnerNorApproved(err) => err.encode(),
            ERC721Error::TransferFromIncorrectOwner(err) => err.encode(),
            ERC721Error::TransferToNonERC721ReceiverImplementer(err) => err.encode(),
            ERC721Error::TransferToZeroAddress(err) => err.encode(),
            ERC721Error::URIQueryForNonexistentToken(err) => err.encode(),
            ERC721Error::ExternalCall(err) => err.into(),
        }
    }
}

/// Simplifies the result type for the contract's methods.
type Result<T, E = ERC721Error> = core::result::Result<T, E>;

// These methods aren't external, but are helpers used by external methods.
// Methods marked as "pub" here are usable outside of the erc721 module (i.e. they're callable from main.rs).
impl<T: ERC721Params> ERC721<T> {
    fn _start_token_id(&self) -> U256 {
        return U256::from(0);
    }

    fn _total_minted(&self) -> U256 {
        return self._current_index.get() - self._start_token_id();
    }

    fn _number_minted(&self, owner: Address) -> U256 {
        return U256::from(self._address_data.get(owner).number_minted.get());
    }

    fn _number_burned(&self, owner: Address) -> U256 {
        return U256::from(self._address_data.get(owner).number_burned.get());
    }

    fn _get_aux(&self, owner: Address) -> U64 {
        return self._address_data.get(owner).aux.get();
    }

    fn _set_aux(&mut self, owner: Address, aux: U64) {
        let mut setter = self._address_data.setter(owner);
        setter.aux.set(aux)
    }

    fn _ownership_of(&self, token_id: U256) -> Result<TokenOwnership> {
        let mut curr = token_id;
        if self._start_token_id() <= curr {
            if curr < self._current_index.get() {
                let mut ownership = self._ownerships.getter(curr);
                if !ownership.burned.get() {
                    if !ownership.addr.is_zero() {
                        unsafe {
                            return Ok(ownership.into_raw());
                        }
                    } else {
                        loop {
                            curr -= U256::from(1);
                            ownership = self._ownerships.getter(curr);
                            if !ownership.addr.is_zero() {
                                unsafe {
                                    return Ok(ownership.into_raw());
                                }
                            }
                        }
                    }
                }
            }
        }
        return Err(ERC721Error::OwnerQueryForNonexistentToken(
            OwnerQueryForNonexistentToken {},
        ));
    }

    fn _base_uri(&self) -> String {
        return "".to_string();
    }

    fn _approve(&mut self, to: Address, token_id: U256, owner: Address) {
        self._token_approvals.setter(token_id).set(to);
        evm::log(Approval {
            owner,
            approved: to,
            token_id,
        });
    }

    fn _exists(&self, token_id: U256) -> bool {
        return self._start_token_id() <= token_id
            && token_id < self._current_index.get()
            && !self._ownerships.get(token_id).burned.get();
    }

    fn _before_token_transfers(
        &self,
        _from: Address,
        _to: Address,
        _start_token_id: U256,
        _quantity: U256,
    ) -> Result<()> {
        Ok(())
    }

    fn _after_token_transfers(
        &self,
        _from: Address,
        _to: Address,
        _start_token_id: U256,
        _quantity: U256,
    ) -> Result<()> {
        Ok(())
    }

    pub fn _mint(&mut self, to: Address, quantity: U256) -> Result<()> {
        let start_token_id = self._current_index.get();
        if to.is_zero() {
            return Err(ERC721Error::MintToZeroAddress(MintToZeroAddress {}));
        }
        if quantity == U256::from(0) {
            return Err(ERC721Error::MintZeroQuantity(MintZeroQuantity {}));
        }

        self._before_token_transfers(Address::default(), to, start_token_id, quantity)?;

        let mut address_data_setter = self._address_data.setter(to);
        let old_balance = address_data_setter.balance.get();
        address_data_setter
            .balance
            .set(old_balance + U64::from(quantity));
        let old_number_minted = address_data_setter.number_minted.get();
        address_data_setter
            .number_minted
            .set(old_number_minted + U64::from(quantity));

        let mut ownership_setter = self._ownerships.setter(start_token_id);
        ownership_setter.addr.set(to);
        ownership_setter
            .start_timestamp
            .set(U64::from(block::timestamp()));

        let mut updated_index = start_token_id;
        let end = updated_index + quantity;

        loop {
            evm::log(Transfer {
                from: Address::default(),
                to,
                token_id: updated_index,
            });
            updated_index += U256::from(1);

            if updated_index >= end {
                break;
            }
        }

        self._current_index.set(updated_index);

        self._after_token_transfers(Address::default(), to, start_token_id, quantity)?;

        Ok(())
    }

    pub fn _safe_mint_with_data<S: TopLevelStorage + BorrowMut<Self>>(
        storage: &mut S,
        to: Address,
        quantity: U256,
        data: Vec<u8>,
    ) -> Result<()> {
        storage.borrow_mut()._mint(to, quantity)?;

        // Equivalent to `to.has_code()`
        let hash = to.codehash();
        if !hash.is_zero()
            && hash != b256!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
        {
            let end = storage.borrow_mut()._current_index.get();
            let mut index = end - quantity;
            loop {
                Self::_check_contract_on_erc721_received(
                    storage,
                    index,
                    Address::default(),
                    to,
                    data.clone(),
                )?;
                index += U256::from(1);
                if index >= end {
                    break;
                }
            }

            if storage.borrow_mut()._current_index.get() != end {
                return Err(ERC721Error::ReentrancyGuard(ReentrancyGuard {}));
            }
        }

        Ok(())
    }

    pub fn _safe_mint<S: TopLevelStorage + BorrowMut<Self>>(
        storage: &mut S,
        to: Address,
        quantity: U256,
    ) -> Result<()> {
        Self::_safe_mint_with_data(storage, to, quantity, Vec::new())?;
        Ok(())
    }

    pub fn _transfer(&mut self, from: Address, to: Address, token_id: U256) -> Result<()> {
        let prev_ownership = self._ownership_of(token_id)?;
        if prev_ownership.addr.get() != from {
            return Err(ERC721Error::TransferFromIncorrectOwner(
                TransferFromIncorrectOwner {},
            ));
        }

        let is_approved_or_owner = msg::sender() == from
            || self.is_approved_for_all(from, msg::sender())?
            || self.get_approved(token_id)? == msg::sender();

        if !is_approved_or_owner {
            return Err(ERC721Error::TransferCallerNotOwnerNorApproved(
                TransferCallerNotOwnerNorApproved {},
            ));
        }
        if to.is_zero() {
            return Err(ERC721Error::TransferToZeroAddress(TransferToZeroAddress {}));
        }

        self._before_token_transfers(from, to, token_id, U256::from(1))?;

        // Clear approvals from the previous owner
        self._approve(to, token_id, from);

        // Deduct from sender balance
        let mut from_address_data_setter = self._address_data.setter(from);
        let old_from_balance = from_address_data_setter.balance.get();
        from_address_data_setter
            .balance
            .set(old_from_balance - U64::from(1));

        // Add to receiver balance
        let mut to_address_data_setter = self._address_data.setter(to);
        let old_to_balance = to_address_data_setter.balance.get();
        to_address_data_setter
            .balance
            .set(old_to_balance + U64::from(1));

        let mut curr_slot = self._ownerships.setter(token_id);
        curr_slot.addr.set(to);
        curr_slot.start_timestamp.set(U64::from(block::timestamp()));

        // If the ownership slot of tokenId+1 is not explicitly set, that means the transfer initiator owns it.
        // Set the slot of tokenId+1 explicitly in storage to maintain correctness for ownerOf(tokenId+1) calls.
        let next_token_id = token_id + U256::from(1);
        let mut next_slot_setter = self._ownerships.setter(next_token_id);
        if next_slot_setter.addr.get().is_zero() {
            // This will suffice for checking _exists(nextTokenId),
            // as a burned slot cannot contain the zero address.
            if next_token_id != self._current_index.get() {
                next_slot_setter.addr.set(from);
                next_slot_setter
                    .start_timestamp
                    .set(U64::from(prev_ownership.start_timestamp.get()));
            }
        }

        evm::log(Transfer { from, to, token_id });

        self._after_token_transfers(from, to, token_id, U256::from(1))?;
        Ok(())
    }

    fn _check_contract_on_erc721_received<S: TopLevelStorage>(
        storage: &mut S,
        token_id: U256,
        from: Address,
        to: Address,
        data: Vec<u8>,
    ) -> Result<()> {
        let receiver = IERC721TokenReceiver::new(to);
        let received = receiver
            .on_erc_721_received(storage, msg::sender(), from, token_id, data)?
            .0;

        if u32::from_be_bytes(received) != ERC721_TOKEN_RECEIVER_ID {
            return Err(ERC721Error::TransferToNonERC721ReceiverImplementer(
                TransferToNonERC721ReceiverImplementer {},
            ));
        }
        Ok(())
    }

    pub fn safe_transfer<S: TopLevelStorage + BorrowMut<Self>>(
        storage: &mut S,
        token_id: U256,
        from: Address,
        to: Address,
        data: Vec<u8>,
    ) -> Result<()> {
        storage.borrow_mut().transfer_from(from, to, token_id)?;
        Self::_check_contract_on_erc721_received(storage, token_id, from, to, data)
    }

    pub fn _burn(&mut self, token_id: U256, approval_check: bool) -> Result<()> {
        let prev_ownership = self._ownership_of(token_id)?;
        let from = prev_ownership.addr.get();

        if approval_check {
            let is_approved_or_owner = msg::sender() == from
                || self.is_approved_for_all(from, msg::sender())?
                || self.get_approved(token_id)? == msg::sender();

            if !is_approved_or_owner {
                return Err(ERC721Error::TransferCallerNotOwnerNorApproved(
                    TransferCallerNotOwnerNorApproved {},
                ));
            }
        }

        self._before_token_transfers(from, Address::default(), token_id, U256::from(1))?;

        // Clear approvals from the previous owner
        self._approve(Address::default(), token_id, from);

        let mut address_data_setter = self._address_data.setter(from);
        let old_balance = address_data_setter.balance.get();
        address_data_setter.balance.set(old_balance - U64::from(1));
        let old_number_burned = address_data_setter.number_burned.get();
        address_data_setter
            .number_burned
            .set(old_number_burned + U64::from(1));

        // Keep track of who burned the token, and the timestamp of burning.
        let mut curr_slot = self._ownerships.setter(token_id);
        curr_slot.addr.set(from);
        curr_slot.start_timestamp.set(U64::from(block::timestamp()));
        curr_slot.burned.set(true);

        // If the ownership slot of tokenId+1 is not explicitly set, that means the burn initiator owns it.
        // Set the slot of tokenId+1 explicitly in storage to maintain correctness for ownerOf(tokenId+1) calls.
        let next_token_id = token_id + U256::from(1);
        let mut next_slot = self._ownerships.setter(next_token_id);
        if next_slot.addr.get().is_zero() {
            // This will suffice for checking _exists(nextTokenId),
            // as a burned slot cannot contain the zero address.
            if next_token_id != self._current_index.get() {
                next_slot.addr.set(from);
                next_slot
                    .start_timestamp
                    .set(prev_ownership.start_timestamp.get());
            }
        }

        evm::log(Transfer {
            from,
            to: Address::default(),
            token_id,
        });

        self._after_token_transfers(from, Address::default(), token_id, U256::from(1))?;

        let old_burn_counter = self._burn_counter.get();
        self._burn_counter.set(old_burn_counter + U256::from(1));

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
    /// Total supply of the collection
    pub fn total_supply(&self) -> Result<U256> {
        Ok(self._current_index.get() - self._burn_counter.get() - self._start_token_id())
    }

    /// Wether the NFT supports a given standard.
    pub fn supports_interface(interface: [u8; 4]) -> Result<bool> {
        if interface == [0xff; 4] {
            // special cased in the ERC165 standard
            return Ok(false);
        }

        const IERC165: u32 = 0x01ffc9a7;
        const IERC721: u32 = 0x80ac58cd;
        const IERC721METADATA: u32 = 0x5b5e139f;

        Ok(matches!(
            u32::from_be_bytes(interface),
            IERC165 | IERC721 | IERC721METADATA
        ))
    }

    /// Gets the number of NFTs owned by an account.
    pub fn balance_of(&self, owner: Address) -> Result<U256> {
        if owner.is_zero() {
            return Err(ERC721Error::BalanceQueryForZeroAddress(
                BalanceQueryForZeroAddress {},
            ));
        }
        Ok(U256::from(self._address_data.get(owner).balance.get()))
    }

    /// Gets the owner of the NFT, if it exists.
    pub fn owner_of(&self, token_id: U256) -> Result<Address> {
        Ok(self._ownership_of(token_id)?.addr.get())
    }

    /// Immutable NFT name.
    pub fn name() -> Result<String> {
        Ok(T::NAME.into())
    }

    /// Immutable NFT symbol.
    pub fn symbol() -> Result<String> {
        Ok(T::SYMBOL.into())
    }

    /// The NFT's Uniform Resource Identifier.
    #[selector(name = "tokenURI")]
    pub fn token_uri(&self, token_id: U256) -> Result<String> {
        self.owner_of(token_id)?; // require NFT exist
        Ok(T::token_uri(token_id))
    }

    pub fn approve(&mut self, to: Address, token_id: U256) -> Result<()> {
        let owner = self.owner_of(token_id)?;
        if to == owner {
            return Err(ERC721Error::ApprovalToCurrentOwner(
                ApprovalToCurrentOwner {},
            ));
        }

        if msg::sender() != owner {
            if !self.is_approved_for_all(owner, msg::sender())? {
                return Err(ERC721Error::ApprovalCallerNotOwnerNorApproved(
                    ApprovalCallerNotOwnerNorApproved {},
                ));
            }
        }

        Ok(self._approve(to, token_id, owner))
    }

    /// Gets the account managing an NFT, or zero if unmanaged.
    pub fn get_approved(&self, token_id: U256) -> Result<Address> {
        if !self._exists(token_id) {
            return Err(ERC721Error::ApprovalQueryForNonexistentToken(
                ApprovalQueryForNonexistentToken {},
            ));
        }
        Ok(self._token_approvals.get(token_id))
    }

    /// Transfers an NFT, but only after checking the `to` address can receive the NFT.
    pub fn safe_transfer_from<S: TopLevelStorage + BorrowMut<Self>>(
        storage: &mut S,
        from: Address,
        to: Address,
        token_id: U256,
    ) -> Result<()> {
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
    ) -> Result<()> {
        storage.borrow_mut()._transfer(from, to, token_id)?;

        // Equivalent to `to.has_code()`
        let hash = to.codehash();
        if !hash.is_zero()
            && hash != b256!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
        {
            Self::_check_contract_on_erc721_received(storage, token_id, from, to, data.to_vec())?
        }
        Ok(())
    }

    /// Transfers the NFT.
    pub fn transfer_from(&mut self, from: Address, to: Address, token_id: U256) -> Result<()> {
        self._transfer(from, to, token_id)?;
        Ok(())
    }

    /// Grants an account the ability to manage all of the sender's NFTs.
    pub fn set_approval_for_all(&mut self, operator: Address, approved: bool) -> Result<()> {
        self._operator_approvals
            .setter(msg::sender())
            .insert(operator, approved);
        evm::log(ApprovalForAll {
            owner: msg::sender(),
            operator,
            approved,
        });
        Ok(())
    }

    /// Determines if an account has been authorized to managing all of a user's NFTs.
    pub fn is_approved_for_all(&self, owner: Address, operator: Address) -> Result<bool> {
        Ok(self._operator_approvals.getter(owner).get(operator))
    }
}
