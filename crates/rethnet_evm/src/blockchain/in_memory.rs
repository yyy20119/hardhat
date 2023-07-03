use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use hashbrown::HashMap;
use rethnet_eth::{
    block::{Block, PartialHeader},
    trie::KECCAK_NULL_RLP,
    Bytes, B256, B64, U256, U64,
};
use revm::{db::BlockHashRef, primitives::SpecId};

use crate::state::StateDebug;

use super::{Blockchain, BlockchainError};

#[derive(Debug, thiserror::Error)]
pub enum BlockchainCreationError<SE> {
    /// Missing base fee per gas for post-merge blockchain
    #[error("Missing base fee per gas for post-merge blockchain")]
    MissingBaseFee,
    /// Missing prevrandao for post-merge blockchain
    #[error("Missing prevrandao for post-merge blockchain")]
    MissingPrevrandao,
    /// State error
    #[error(transparent)]
    State(SE),
}

#[derive(Debug, thiserror::Error)]
pub enum InsertBlockError {
    #[error("Invalid block numnber: ${actual}. Expected: ${expected}")]
    InvalidBlockNumber { actual: U256, expected: U256 },
}

/// Blockchain that's stored in-memory.
#[derive(Debug)]
pub struct InMemoryBlockchain {
    blocks: Vec<Arc<Block>>,
    hash_to_block: HashMap<B256, Arc<Block>>,
}

impl InMemoryBlockchain {
    /// Constructs a [`InMemoryBlockchain`] using the provided arguments to build a genesis block.
    pub fn new<S: StateDebug>(
        state: &S,
        spec_id: SpecId,
        gas_limit: U256,
        timestamp: Option<U256>,
        prevrandao: Option<B256>,
        base_fee: Option<U256>,
    ) -> Result<Self, BlockchainCreationError<S::Error>> {
        const EXTRA_DATA: &[u8] = b"124";

        let genesis_block = Block::new(
            PartialHeader {
                state_root: state.state_root().map_err(BlockchainCreationError::State)?,
                receipts_root: KECCAK_NULL_RLP,
                difficulty: if spec_id >= SpecId::MERGE {
                    U256::ZERO
                } else {
                    U256::from(1)
                },
                number: U256::ZERO,
                gas_limit,
                gas_used: U256::ZERO,
                timestamp: timestamp.unwrap_or_else(|| {
                    U256::from(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("Current time must be after unix epoch")
                            .as_secs(),
                    )
                }),
                extra_data: Bytes::from(EXTRA_DATA),
                mix_hash: if spec_id >= SpecId::MERGE {
                    prevrandao.ok_or(BlockchainCreationError::MissingPrevrandao)?
                } else {
                    B256::zero()
                },
                nonce: if spec_id >= SpecId::MERGE {
                    B64::ZERO
                } else {
                    B64::from(U64::from(42))
                },
                base_fee: if spec_id >= SpecId::MERGE {
                    Some(base_fee.ok_or(BlockchainCreationError::MissingBaseFee)?)
                } else {
                    None
                },
                ..PartialHeader::default()
            },
            Vec::new(),
            Vec::new(),
        );

        Ok(unsafe { Self::with_genesis_block_unchecked(genesis_block) })
    }

    /// Constructs a new [`InMemoryBlockchain`] with the provided genesis block.
    pub fn with_genesis_block(genesis_block: Block) -> Result<Self, InsertBlockError> {
        if genesis_block.header.number != U256::ZERO {
            return Err(InsertBlockError::InvalidBlockNumber {
                actual: genesis_block.header.number,
                expected: U256::ZERO,
            });
        }

        Ok(unsafe { Self::with_genesis_block_unchecked(genesis_block) })
    }

    /// Inserts a block without checking its validity
    ///
    /// # Safety
    ///
    /// Ensure that the block's parent hash equals that of the last block and its block
    /// number is one higher than that of the last block.
    pub unsafe fn insert_block_unchecked(&mut self, block: Block) {
        let block = Arc::new(block);

        self.blocks.push(block.clone());
        self.hash_to_block.insert(block.header.hash(), block);
    }

    unsafe fn with_genesis_block_unchecked(genesis_block: Block) -> Self {
        let genesis_block = Arc::new(genesis_block);
        let mut hash_to_block = HashMap::new();
        hash_to_block.insert(genesis_block.header.hash(), genesis_block.clone());

        Self {
            blocks: vec![genesis_block],
            hash_to_block,
        }
    }
}

impl Blockchain for InMemoryBlockchain {
    type Error = BlockchainError;

    fn last_block(&self) -> Block {
        self.blocks
            .last()
            .expect("A genesis block is always present")
            .as_ref()
            .clone()
    }

    fn insert_block(&mut self, block: Block) -> Result<(), Self::Error> {
        let last_block = self
            .blocks
            .last()
            .expect("A genesis block is always present");

        let next_block_number = last_block.header.number + U256::from(1);
        if block.header.number != next_block_number {
            return Err(BlockchainError::InvalidBlockNumber {
                actual: block.header.number,
                expected: next_block_number,
            });
        }

        if block.header.parent_hash != last_block.header.hash() {
            return Err(BlockchainError::InvalidParentHash);
        }

        // Safety: We've already performed the checks
        unsafe { self.insert_block_unchecked(block) };

        Ok(())
    }
}

impl BlockHashRef for InMemoryBlockchain {
    type Error = BlockchainError;

    fn block_hash(&self, number: U256) -> Result<B256, Self::Error> {
        // Question: Do we need to support block number larger than u64::MAX
        if number > U256::from(u64::MAX) {
            return Err(BlockchainError::BlockNumberTooLarge);
        }

        let number = usize::try_from(number.as_limbs()[0])
            .map_err(|_| BlockchainError::BlockNumberTooLarge)?;

        self.blocks
            .get(number)
            .map(|block| block.header.hash())
            .ok_or(BlockchainError::UnknownBlockNumber)
    }
}
