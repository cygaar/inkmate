use stylus_sdk::{alloy_primitives::Address, call::RawCall};

const HASH_OUTPUT_SIZE: usize = 32;

const NUM_BYTES_SIGNATURE: usize = 65;

/// The number of bytes it takes to represent an Ethereum address
const NUM_BYTES_ADDRESS: usize = 20;

const EC_RECOVER_INPUT_LEN: usize = 128;

/// The number of bytes it takes to represent an unsigned 256-bit integer
pub const NUM_BYTES_U256: usize = 32;

/// The last byte of the `ecRecover` precompile address, 0x01
pub const EC_RECOVER_ADDRESS_LAST_BYTE: u8 = 1;

/// An error that occurs during ECDSA verification
#[derive(Debug)]
pub struct EcdsaError;

pub fn ec_recover(
    message_hash: &[u8; HASH_OUTPUT_SIZE],
    v: u8,
    r: &[u8; NUM_BYTES_U256],
    s: &[u8; NUM_BYTES_U256],
) -> Result<[u8; NUM_BYTES_ADDRESS], EcdsaError> {
    // Prepare the input data for the `ecRecover` precompile, namely:
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

    // Call the `ecRecover` precompile
    let res = RawCall::new_static()
        // Only get the last 20 bytes of the 32-byte return data
        .limit_return_data(NUM_BYTES_U256 - NUM_BYTES_ADDRESS, NUM_BYTES_ADDRESS)
        .call(
            Address::with_last_byte(EC_RECOVER_ADDRESS_LAST_BYTE),
            &input,
        )
        .map_err(|_| EcdsaError)?;

    res.try_into().map_err(|_| EcdsaError)
}
