use crate::events::spot_entry::SpotEntry;

use std::fmt;

pub(crate) struct Transaction {
    pub(crate) block_number: u64,
    pub(crate) transaction_hash: String,
    pub(crate) from_address: String,
    pub(crate) spot_entry: SpotEntry
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ {} [{}] <-[{}] : {} }}",
            self.block_number,
            self.transaction_hash,
            self.from_address,
            self.spot_entry
        )
    }
}