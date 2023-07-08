mod fork;
mod in_memory;

use std::fmt::Debug;

use rethnet_eth::{block::Block, U256};
use revm::db::BlockHashRef;

pub use self::{fork::ForkBlockchain, in_memory::InMemoryBlockchain};

/// Combinatorial error for the blockchain API.
#[derive(Debug, thiserror::Error)]
pub enum BlockchainError {
    /// Block number exceeds storage capacity (usize::MAX)
    #[error("Block number exceeds storage capacity.")]
    BlockNumberTooLarge,
    /// Invalid block number
    #[error("Invalid block numnber: ${actual}. Expected: ${expected}.")]
    InvalidBlockNumber {
        /// Provided block number
        actual: U256,
        /// Expected block number
        expected: U256,
    },
    /// Invalid parent hash
    #[error("Invalid parent hash")]
    InvalidParentHash,
    /// Block number does not exist in blockchain
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
