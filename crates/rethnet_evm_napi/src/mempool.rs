use std::{ops::Deref, sync::Arc};

use napi::{bindgen_prelude::Buffer, tokio::sync::RwLock, Status};
use napi_derive::napi;
use rethnet_eth::B256;

use crate::{state::StateManager, transaction::PendingTransaction};

/// The mempool contains transactions pending inclusion in the blockchain.
#[napi]
pub struct MemPool {
    inner: Arc<RwLock<rethnet_evm::MemPool>>,
}

impl Deref for MemPool {
    type Target = Arc<RwLock<rethnet_evm::MemPool>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[napi]
impl MemPool {
    #[doc = "Constructs a new [`MemPool`]."]
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(rethnet_evm::MemPool::default())),
        }
    }

    #[doc = "Tries to add the provided transaction to the instance."]
    #[napi]
    pub async fn add_transaction(
        &self,
        state_manager: &StateManager,
        transaction: &PendingTransaction,
    ) -> napi::Result<()> {
        let state = state_manager.read().await;

        self.write()
            .await
            .add_transaction(&*state, (*transaction).clone())
            .map_err(|e| napi::Error::new(Status::GenericFailure, e.to_string()))
    }

    #[doc = "Removes the transaction corresponding to the provided hash, if it exists."]
    #[napi]
    pub async fn remove_transaction(&self, hash: Buffer) {
        let hash = B256::from_slice(&hash);

        self.write().await.remove_transaction(&hash)
    }

    #[doc = "Updates the instance, moving any future transactions to the pending status, if their nonces are high enough."]
    #[napi]
    pub async fn update(&self, state_manager: &StateManager) -> napi::Result<()> {
        let state = state_manager.read().await;

        self.write()
            .await
            .update(&*state)
            .map_err(|e| napi::Error::new(Status::GenericFailure, e.to_string()))
    }

    #[doc = "Returns all pending transactions, for which the nonces are guaranteed to be high enough."]
    #[napi]
    pub async fn pending_transactions(&self) -> Vec<PendingTransaction> {
        self.read()
            .await
            .pending_transactions()
            .cloned()
            .map(PendingTransaction::from)
            .collect()
    }
}
