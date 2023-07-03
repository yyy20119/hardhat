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

impl Default for MemPool {
    fn default() -> Self {
        Self::new()
    }
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
        Self::default()
    }

    #[doc = "Creates a deep clone of the [`MemPool`]"]
    #[napi]
    pub async fn deep_clone(&self) -> Self {
        let mem_pool = self.read().await;

        Self {
            inner: Arc::new(RwLock::new(mem_pool.clone())),
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
    pub async fn remove_transaction(&self, hash: Buffer) -> bool {
        let hash = B256::from_slice(&hash);

        self.write().await.remove_transaction(&hash).is_some()
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

    #[doc = "Returns all transactions in the mem pool."]
    #[napi]
    pub async fn transactions(&self) -> Vec<PendingTransaction> {
        let mempool = self.read().await;

        mempool
            .pending_transactions()
            .iter()
            .chain(mempool.future_transactions().iter())
            .cloned()
            .map(PendingTransaction::from)
            .collect()
    }

    #[doc = "Returns all future transactions, for which the nonces are too high."]
    #[napi]
    pub async fn future_transactions(&self) -> Vec<PendingTransaction> {
        self.read()
            .await
            .future_transactions()
            .iter()
            .cloned()
            .map(PendingTransaction::from)
            .collect()
    }

    #[doc = "Returns all pending transactions, for which the nonces are guaranteed to be high enough."]
    #[napi]
    pub async fn pending_transactions(&self) -> Vec<PendingTransaction> {
        self.read()
            .await
            .pending_transactions()
            .iter()
            .cloned()
            .map(PendingTransaction::from)
            .collect()
    }

    #[doc = "Returns whether the [`MemPool`] contains any future transactions."]
    #[napi]
    pub async fn has_future_transactions(&self) -> bool {
        !self.read().await.future_transactions().is_empty()
    }

    #[doc = "Returns whether the [`MemPool`] contains any pending transactions."]
    #[napi]
    pub async fn has_pending_transactions(&self) -> bool {
        !self.read().await.pending_transactions().is_empty()
    }

    #[doc = "Returns the pending transaction corresponding to the provided hash, if it exists."]
    #[napi]
    pub async fn transaction_by_hash(&self, hash: Buffer) -> Option<PendingTransaction> {
        let hash = B256::from_slice(&hash);

        self.read()
            .await
            .transaction_by_hash(&hash)
            .cloned()
            .map(PendingTransaction::from)
    }
}
