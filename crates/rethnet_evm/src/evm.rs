use std::fmt::Debug;

use revm::{BlockEnv, CfgEnv, TxEnv};

use crate::{blockchain::AsyncBlockchain, db::AsyncDatabase};

/// Creates an evm from the provided database, config, transaction, and block.
#[allow(clippy::type_complexity)]
pub fn build_evm<'b, 'd, BE, DE>(
    blockchain: &'b AsyncBlockchain<BE>,
    db: &'d AsyncDatabase<DE>,
    cfg: CfgEnv,
    transaction: TxEnv,
    block: BlockEnv,
) -> revm::EVM<&'d AsyncDatabase<DE>, &'b AsyncBlockchain<BE>>
where
    BE: Debug + Send + 'static,
    DE: Debug + Send + 'static,
{
    let mut evm = revm::EVM::new();
    evm.set_blockchain(blockchain);
    evm.database(db);
    evm.env.cfg = cfg;
    evm.env.block = block;
    evm.env.tx = transaction;

    evm
}
