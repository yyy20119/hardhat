use std::sync::Arc;

use hashbrown::HashMap;
use rethnet_eth::{block::Block, B256, U256};
use revm::db::BlockHashRef;

use super::{Blockchain, BlockchainError};

#[derive(Debug, thiserror::Error)]
pub enum InsertBlockError {
    #[error("Invalid block numnber: ${actual}. Expected: ${expected}")]
    InvalidBlockNumber { actual: U256, expected: U256 },
}

#[derive(Debug)]
pub struct InMemoryBlockchain {
    blocks: Vec<Arc<Block>>,
    hash_to_block: HashMap<B256, Arc<Block>>,
}

impl InMemoryBlockchain {
    /// Constructs a a new [`InMemoryBlockchain`].
    pub fn new(genesis_block: Block) -> Result<Self, InsertBlockError> {
        if genesis_block.header.number != U256::ZERO {
            return Err(InsertBlockError::InvalidBlockNumber {
                actual: genesis_block.header.number,
                expected: U256::ZERO,
            });
        }

        let genesis_block = Arc::new(genesis_block);
        let mut hash_to_block = HashMap::new();
        hash_to_block.insert(genesis_block.header.hash(), genesis_block.clone());

        Ok(Self {
            blocks: vec![genesis_block],
            hash_to_block,
        })
    }

    pub unsafe fn insert_block_unchecked(&mut self, block: Block) {
        let block = Arc::new(block);

        self.blocks.push(block.clone());
        self.hash_to_block.insert(block.header.hash(), block);
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
        Ok(unsafe { self.insert_block_unchecked(block) })
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
