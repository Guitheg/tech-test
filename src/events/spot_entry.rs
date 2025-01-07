use std::fmt;

use starknet::core::types::Felt;

pub(crate) struct SpotEntry {
    pub(crate) timestamp: u64,
    pub(crate) source: String,
    pub(crate) publisher: String,
    pub(crate) price: u128,
    pub(crate) pair_id: String,
    pub(crate) volume: u128
}

impl fmt::Display for SpotEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpotEntry {{ timestamp: {}, source: {}, publisher: {}, price: {}, pair_id: {}, volume: {} }}",
            self.timestamp,
            self.source,
            self.publisher,
            self.price,
            self.pair_id,
            self.volume
        )
    }
}

pub(crate) fn felt_to_u128(felt: Felt) -> Result<u128, String> {
    u128::from_str_radix(felt.to_hex_string().trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to convert felt to u128: {e}"))
}

pub(crate) fn felt_to_utf8_str(felt: Felt) -> Result<String, String> {
    match String::from_utf8(felt.to_bytes_be().to_vec()) {
        Ok(str) => Ok(str.trim_matches('\0').to_string()),
        Err(e) => Err(format!("Failed to convert felt to UTF8: {e}"))
    }
}