mod in_memory;

use std::fmt::Debug;

use rethnet_eth::{block::Block, U256};
use revm::db::BlockHashRef;

pub use in_memory::InMemoryBlockchain;

#[derive(Debug, thiserror::Error)]
pub enum BlockchainError {
    #[error("Block number exceeds storage capacity.")]
    BlockNumberTooLarge,
    #[error("Invalid block numnber: ${actual}. Expected: ${expected}.")]
    InvalidBlockNumber { actual: U256, expected: U256 },
    #[error("Invalid parent hash")]
    InvalidParentHash,
    #[error("Unknown block number")]
    UnknownBlockNumber,
}

/// Trait for implementations of an Ethereum blockchain.
pub trait Blockchain {
    /// The blockchain's error type
    type Error;

    /// Returns the last block in the blockchain.
    // TODO: Make this a reference when we no longer support napi
    fn last_block(&self) -> Block;

    /// Inserts the provided block into the blockchain.
    fn insert_block(&mut self, block: Block) -> Result<(), Self::Error>;
}

/// Trait that meets all requirements for a synchronous database that can be used by [`AsyncBlockchain`].
pub trait SyncBlockchain<E>:
    Blockchain<Error = E> + BlockHashRef<Error = E> + Send + Sync + Debug + 'static
where
    E: Debug + Send,
{
}

impl<B, E> SyncBlockchain<E> for B
where
    B: Blockchain<Error = E> + BlockHashRef<Error = E> + Send + Sync + Debug + 'static,
    E: Debug + Send,
{
}
