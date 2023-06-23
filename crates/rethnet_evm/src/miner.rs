use std::{collections::VecDeque, fmt::Debug, sync::Arc};

use rethnet_eth::{
    block::{Block, Header},
    receipt::TypedReceipt,
    Address, B64, U256, U64,
};
use revm::primitives::{CfgEnv, ExecutionResult, SpecId};
use tokio::sync::RwLock;

use crate::{
    block::{BlockBuilderCreationError, BlockResult},
    blockchain::SyncBlockchain,
    state::SyncState,
    trace::{Trace, TraceCollector},
    BlockBuilder, BlockOptions, BlockTransactionError, MemPool, RandomHashGenerator,
};

/// The result of mining a block.
pub struct MineBlockResult {
    /// Mined block
    block: Block,
    /// Transaction results
    transaction_results: Vec<ExecutionResult>,
    /// Transaction receipts
    transaction_receipts: Vec<TypedReceipt>,
    /// Transaction traces
    transaction_traces: Vec<Trace>,
}

/// An error that occurred while mining a block.
#[derive(Debug, thiserror::Error)]
pub enum MineBlockError<BE, SE> {
    /// An error that occurred while aborting the block builder.
    #[error(transparent)]
    BlockAbort(SE),
    /// An error that occurred while constructing a block builder.
    #[error(transparent)]
    BlockBuilderCreation(#[from] BlockBuilderCreationError<SE>),
    /// An error that occurred while executing a transaction.
    #[error(transparent)]
    BlockTransaction(#[from] BlockTransactionError<BE, SE>),
    /// An error that occurred while finalizing a block.
    #[error(transparent)]
    BlockFinalize(SE),
    /// A blockchain error
    #[error(transparent)]
    Blockchain(BE),
    /// An error that occurred while updating the transaction pool.
    #[error(transparent)]
    TransactionPoolUpdate(SE),
}

/// Type for mining blocks.
pub struct BlockMiner<BE, SE>
where
    BE: Debug + Send + 'static,
    SE: Debug + Send + 'static,
{
    blockchain: Arc<RwLock<Box<dyn SyncBlockchain<BE>>>>,
    state: Arc<RwLock<Box<dyn SyncState<SE>>>>,
    transaction_pool: Arc<RwLock<MemPool>>,
    prevrandao_generator: RandomHashGenerator,
    cfg: CfgEnv,
    block_gas_limit: U256,
    beneficiary: Address,
}

impl<BE, SE> BlockMiner<BE, SE>
where
    BE: Debug + Send + 'static,
    SE: Debug + Send + 'static,
{
    /// Mines a block using as many transactions as can fit in it.
    pub async fn mine_block(
        &mut self,
        timestamp: U256,
        reward: U256,
        base_fee: Option<U256>,
    ) -> Result<MineBlockResult, MineBlockError<BE, SE>> {
        let mut block_builder = {
            let blockchain = self.blockchain.read().await;
            let parent_block = blockchain.last_block();

            BlockBuilder::new(
                self.blockchain.clone(),
                self.state.clone(),
                self.cfg.clone(),
                parent_block.header.clone(),
                BlockOptions {
                    beneficiary: Some(self.beneficiary),
                    number: Some(blockchain.last_block().header.number.clone()),
                    gas_limit: Some(self.block_gas_limit),
                    timestamp: Some(timestamp),
                    mix_hash: if self.cfg.spec_id >= SpecId::MERGE {
                        Some(self.prevrandao_generator.next_value())
                    } else {
                        None
                    },
                    nonce: Some(if self.cfg.spec_id >= SpecId::MERGE {
                        B64::ZERO
                    } else {
                        B64::from(U64::from(42))
                    }),
                    base_fee: if self.cfg.spec_id >= SpecId::LONDON {
                        Some(
                            base_fee
                                .unwrap_or_else(|| calculate_next_base_fee(&parent_block.header)),
                        )
                    } else {
                        None
                    },
                    ..Default::default()
                },
            )
            .await?
        };

        let mut transaction_pool = self.transaction_pool.write().await;
        let mut pending_transactions: VecDeque<_> =
            transaction_pool.pending_transactions().cloned().collect();

        let mut results = Vec::new();
        let mut traces = Vec::new();

        while let Some(transaction) = pending_transactions.pop_front() {
            let mut tracer = TraceCollector::default();

            let transaction_hash = transaction.hash().clone();

            match block_builder
                .add_transaction(transaction, Some(&mut tracer))
                .await
            {
                // Question: Should we check this condition manually, to minimise overhead?
                Err(BlockTransactionError::ExceedsBlockGasLimit) => continue,
                Err(e) => {
                    block_builder
                        .abort()
                        .await
                        .map_err(MineBlockError::BlockAbort)?;

                    return Err(MineBlockError::BlockTransaction(e));
                }
                Ok(result) => {
                    results.push(result);
                    traces.push(tracer.into_trace());

                    transaction_pool.remove_transaction(&transaction_hash);
                }
            }
        }

        let rewards = vec![(self.beneficiary, reward)];
        let BlockResult {
            block,
            receipts,
            callers: _callers,
        } = block_builder
            .finalize(rewards, None)
            .await
            .map_err(MineBlockError::BlockFinalize)?;

        self.blockchain
            .write()
            .await
            .insert_block(block.clone())
            .map_err(MineBlockError::Blockchain)?;

        transaction_pool
            .update(&*self.state.read().await)
            .map_err(MineBlockError::TransactionPoolUpdate)?;

        Ok(MineBlockResult {
            block,
            transaction_results: results,
            transaction_receipts: receipts,
            transaction_traces: traces,
        })
    }
}

/// Calculates the next base fee for a post-London block, given the parent's header.
fn calculate_next_base_fee(parent: &Header) -> U256 {
    let elasticity = U256::from(2);
    let base_fee_max_change_denominator = U256::from(8);

    let parent_gas_target = parent.gas_limit / elasticity;
    let parent_base_fee = parent
        .base_fee_per_gas
        .expect("Post-London headers must contain a baseFee");

    match parent.gas_used.cmp(&parent_gas_target) {
        std::cmp::Ordering::Less => {
            let gas_used_delta = parent_gas_target - parent.gas_used;

            let delta = parent_base_fee * gas_used_delta
                / parent_gas_target
                / base_fee_max_change_denominator;

            (parent_base_fee - delta).max(U256::ZERO)
        }
        std::cmp::Ordering::Equal => parent_base_fee,
        std::cmp::Ordering::Greater => {
            let gas_used_delta = parent.gas_used - parent_gas_target;

            let delta = parent_base_fee * gas_used_delta
                / parent_gas_target
                / base_fee_max_change_denominator;
            if delta > U256::from(1) {
                delta
            } else {
                parent_base_fee + U256::from(1)
            }
        }
    }
}
