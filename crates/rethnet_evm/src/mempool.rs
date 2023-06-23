use rethnet_eth::B256;
use revm::db::StateRef;

use crate::PendingTransaction;

/// The mempool contains transactions pending inclusion in the blockchain.
#[derive(Default)]
pub struct MemPool {
    /// Transactions that can be executed now
    pending_transactions: Vec<PendingTransaction>,
    /// Transactions that can be executed in the future, once the nonce is high enough
    future_transactions: Vec<PendingTransaction>,
}

impl MemPool {
    /// Tries to add the provided transaction to the [`Pool`].
    pub fn add_transaction<S: StateRef>(
        &mut self,
        state: &S,
        transaction: PendingTransaction,
    ) -> Result<(), S::Error> {
        self.add_transaction_impl(state, transaction.into())
    }

    /// Removes the transaction corresponding to the provided transaction hash, if it exists.
    pub fn remove_transaction(&mut self, hash: &B256) {
        self.pending_transactions
            .retain(|transaction| *transaction.hash() != *hash);
    }

    /// Updates the [`Pool`], moving any future transactions to the pending status, if their nonces are high enough.
    pub fn update<S: StateRef>(&mut self, state: &S) -> Result<(), S::Error> {
        let mut future_transactions = Vec::with_capacity(self.future_transactions.capacity());
        std::mem::swap(&mut self.future_transactions, &mut future_transactions);

        for transaction in future_transactions.into_iter() {
            self.add_transaction_impl(state, transaction)?;
        }

        Ok(())
    }

    /// Returns all pending transactions, for which the nonces are guaranteed to be high enough.
    pub fn pending_transactions(&self) -> impl Iterator<Item = &PendingTransaction> {
        self.pending_transactions.iter()
    }

    fn add_transaction_impl<S: StateRef>(
        &mut self,
        state: &S,
        transaction: PendingTransaction,
    ) -> Result<(), S::Error> {
        let account = state.basic(*transaction.caller())?;

        // Question: Must the account exist?
        let account = account.unwrap_or_default();
        if *transaction.nonce() > account.nonce {
            self.future_transactions.push(transaction);
        } else {
            self.pending_transactions.push(transaction);
        }

        Ok(())
    }
}
