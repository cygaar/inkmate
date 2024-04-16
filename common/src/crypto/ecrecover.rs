//! Defines the ecrecover trait which calls an implementation that can implemented
//! through a precompile call or natively in rust. The native rust implementation is only
//! used for testing purposes as the binary size for the compiled WASM is too large for Stylus.
//!
//! This code is based off of Renegade's implementation:
//! https://github.com/renegade-fi/renegade-contracts/blob/main/contracts-stylus/src/utils/backends.rs#L115

/// The number of bytes in a hash digest used by the transcript
pub const HASH_OUTPUT_SIZE: usize = 32;

/// The number of bytes it takes to represent an Ethereum address
pub const NUM_BYTES_ADDRESS: usize = 20;

/// The byte length of the input to the `ecRecover` precompile
pub const EC_RECOVER_INPUT_LEN: usize = 128;

/// The number of bytes it takes to represent an unsigned 256-bit integer
pub const NUM_BYTES_U256: usize = 32;

/// The last byte of the `ecRecover` precompile address, 0x01
pub const EC_RECOVER_ADDRESS_LAST_BYTE: u8 = 1;

/// An error that occurs during ECDSA verification
#[derive(Debug)]
pub struct EcdsaError;

pub trait EcRecoverTrait {
    fn ecrecover(
        message_hash: &[u8; HASH_OUTPUT_SIZE],
        v: u8,
        r: &[u8; NUM_BYTES_U256],
        s: &[u8; NUM_BYTES_U256],
    ) -> Result<[u8; NUM_BYTES_ADDRESS], EcdsaError> {
        // Prepare the input data for the `ecrecover` precompile, namely:
        // input[0..32] = message_hash
        // input[32..64] = v (big-endian)
        // input[64..96] = r (big-endian)
        // input[96..128] = s (big-endian)
        let mut input = [0_u8; EC_RECOVER_INPUT_LEN];
        // Add message hash to input
        input[..NUM_BYTES_U256].copy_from_slice(message_hash);
        // Left-pad `v` with zero-bytes & add to input
        input[NUM_BYTES_U256..2 * NUM_BYTES_U256 - 1].copy_from_slice(&[0_u8; NUM_BYTES_U256 - 1]);
        // We expect `v` to be either 0 or 1, but the `ecRecover`
        // precompile expects either 27 or 28
        if v <= 1 {
            input[2 * NUM_BYTES_U256 - 1] = v + 27;
        } else {
            input[2 * NUM_BYTES_U256 - 1] = v;
        }
        // Add `r` to input
        input[2 * NUM_BYTES_U256..3 * NUM_BYTES_U256].copy_from_slice(r);
        // Add `s` to input
        input[3 * NUM_BYTES_U256..].copy_from_slice(s);

        Self::ecrecover_implementation(input)
    }

    fn ecrecover_implementation(
        input: [u8; EC_RECOVER_INPUT_LEN],
    ) -> Result<[u8; NUM_BYTES_ADDRESS], EcdsaError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{B256, B512};
    use ethers::utils::keccak256;
    use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
    use secp256k1::rand::rngs::OsRng;
    use secp256k1::{Message, Secp256k1};

    struct RustEcRecover;

    impl EcRecoverTrait for RustEcRecover {
        /// ecrecover implementation based off of revm's precompile:
        /// https://github.com/bluealloy/revm/blob/main/crates/precompile/src/secp256k1.rs#L13
        fn ecrecover_implementation(
            input: [u8; EC_RECOVER_INPUT_LEN],
        ) -> Result<[u8; NUM_BYTES_ADDRESS], EcdsaError> {
            // `v` must be a 32-byte big-endian integer equal to 27 or 28.
            if !(input[32..63].iter().all(|&b| b == 0) && matches!(input[63], 27 | 28)) {
                return Ok([0; NUM_BYTES_ADDRESS]);
            }

            let msg = <&B256>::try_from(&input[0..32]).unwrap();
            let mut recid = input[63] - 27;
            let sig = <&B512>::try_from(&input[64..128]).unwrap();

            // parse signature
            let mut sig = Signature::from_slice(sig.as_slice()).map_err(|_| EcdsaError)?;

            // normalize signature and flip recovery id if needed.
            if let Some(sig_normalized) = sig.normalize_s() {
                sig = sig_normalized;
                recid ^= 1;
            }
            let recid = RecoveryId::from_byte(recid).expect("recovery ID is valid");

            // recover key
            let recovered_key = VerifyingKey::recover_from_prehash(&msg[..], &sig, recid)
                .map_err(|_| EcdsaError)?;
            // hash it
            let hash = keccak256(
                &recovered_key
                    .to_encoded_point(/* compress = */ false)
                    .as_bytes()[1..],
            );

            // truncate to 20 bytes
            // hash[..12].fill(0);
            let result: [u8; NUM_BYTES_ADDRESS] = hash[12..].try_into().map_err(|_| EcdsaError)?;
            Ok(result)
        }
    }

    #[test]
    fn test_ec_recover_with_native_implementation() {
        let secp = Secp256k1::new();

        // Generate a new private key
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

        // Create a hash of a message
        let message = "Hello, Stylus!";
        let hash = keccak256(message.as_bytes());
        let msg = Message::from_digest_slice(&hash).unwrap();

        // Parse r, s, v values
        let recoverable_signature = secp.sign_ecdsa_recoverable(&msg, &secret_key);
        let (rec_id, sig_bytes) = recoverable_signature.serialize_compact();
        let mut rec_id = rec_id.to_i32() as u8;
        rec_id += 27;
        let r: [u8; 32] = sig_bytes[0..32]
            .try_into()
            .expect("Slice with incorrect length");
        let s: [u8; 32] = sig_bytes[32..64]
            .try_into()
            .expect("Slice with incorrect length");

        // Test ecrecover function using the native rust implementation
        let result =
            RustEcRecover::ecrecover(&hash, rec_id, &r, &s).expect("Recovery should succeed");
        let expected_address = {
            let recovered_key = public_key.serialize_uncompressed();
            let keccak_hash = keccak256(&recovered_key[1..]); // Skip the first byte
            let mut address = [0u8; 20];
            address.copy_from_slice(&keccak_hash[12..32]);
            address
        };

        assert_eq!(
            result, expected_address,
            "Recovered address should match the expected address"
        );
    }
}
