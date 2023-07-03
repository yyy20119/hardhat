use std::{fmt::Debug, ops::Deref, sync::Arc};

use napi::{bindgen_prelude::ObjectFinalize, tokio::sync::RwLock, Env, Status};
use napi_derive::napi;

use rethnet_evm::blockchain::{BlockchainError, SyncBlockchain};

use crate::block::Block;

// An arbitrarily large amount of memory to signal to the javascript garbage collector that it needs to
// attempt to free the blockchain object's memory.
const BLOCKCHAIN_MEMORY_SIZE: i64 = 10_000;

/// The Rethnet blockchain
#[napi(custom_finalize)]
#[derive(Debug)]
pub struct Blockchain {
    inner: Arc<RwLock<Box<dyn SyncBlockchain<BlockchainError>>>>,
}

impl Blockchain {
    fn with_blockchain<B>(env: &mut Env, blockchain: B) -> napi::Result<Self>
    where
        B: SyncBlockchain<BlockchainError>,
    {
        let blockchain: Box<dyn SyncBlockchain<BlockchainError>> = Box::new(blockchain);

        env.adjust_external_memory(BLOCKCHAIN_MEMORY_SIZE)?;

        Ok(Self {
            inner: Arc::new(RwLock::new(blockchain)),
        })
    }
}

impl Deref for Blockchain {
    type Target = Arc<RwLock<Box<dyn SyncBlockchain<BlockchainError>>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[napi]
impl Blockchain {
    /// Constructs a new blockchain that queries the blockhash using a callback.
    #[napi(factory)]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub fn with_genesis_block(mut env: Env, genesis_block: &Block) -> napi::Result<Self> {
        let blockchain = rethnet_evm::blockchain::InMemoryBlockchain::with_genesis_block(
            (*genesis_block).clone(),
        )
        .map_err(|e| napi::Error::new(Status::InvalidArg, e.to_string()))?;

        Self::with_blockchain(&mut env, blockchain)
    }

    // #[napi]
    // pub async fn insert_block(
    //     &mut self,
    //     block_number: BigInt,
    //     block_hash: Buffer,
    // ) -> napi::Result<()> {
    //     let block_number = BigInt::try_cast(block_number)?;
    //     let block_hash = B256::from_slice(&block_hash);

    //     self.db
    //         .insert_block(block_number, block_hash)
    //         .await
    //         .map_err(|e| napi::Error::new(Status::GenericFailure, e.to_string()))
    // }
}

impl ObjectFinalize for Blockchain {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    fn finalize(self, mut env: Env) -> napi::Result<()> {
        env.adjust_external_memory(-BLOCKCHAIN_MEMORY_SIZE)?;

        Ok(())
    }
}
