use hashbrown::HashMap;
use rethnet_eth::{
    transaction::{SignedTransaction, TransactionRequest},
    Address,
};
use secp256k1::{Secp256k1, SecretKey, VerifyOnly};

/// Error type for signing
#[derive(Debug, thiserror::Error)]
pub enum SignError {
    /// Invalid address for signer
    #[error("Signer for address `{0}` does not exist.")]
    InvalidSigner(Address),
}

pub struct Signer {
    accounts: HashMap<Address, SecretKey>,
    context: Secp256k1<VerifyOnly>,
}

impl Signer {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            context: Secp256k1::verification_only(),
        }
    }

    pub fn sign(&self, request: TransactionRequest, caller: &Address) -> Result<SignedTransaction> {
        let signer = self
            .accounts
            .get(caller)
            .ok_or_else(|| SignError::InvalidSigner(*caller))?;
    }
}
