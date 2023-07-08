use hashbrown::HashMap;
use rethnet_eth::{remote::RpcClient, Address, B256, U256};
use revm::{
    db::BlockHashRef,
    primitives::{AccountInfo, SpecId},
};

use crate::state::StateDebug;

use super::{Blockchain, InMemoryBlockchain};

pub struct ForkBlockchain {
    local_blockchain: InMemoryBlockchain,
    rpc_client: RpcClient,
    fork_block_number: U256,
}

impl ForkBlockchain {
    pub fn new<S: StateDebug>(
        state: &S,
        spec_id: SpecId,
        remote_url: &str,
        fork_block_number: U256,
        genesis_accounts: HashMap<Address, AccountInfo>,
    ) -> Result<Self, S::Error> {
        let rpc_client = RpcClient::new(remote_url);

        let network_id = rpc_client.network_id().await?;

        let local_blockchain = InMemoryBlockchain::new(state, spec_id)?;

        Ok(Self {
            local_blockchain,
            rpc_client,
            fork_block_number,
        })
    }
}

impl BlockHashRef for ForkBlockchain {
    type Error;

    fn block_hash(&self, number: U256) -> Result<B256, Self::Error> {
        todo!()
    }
}

impl Blockchain for ForkBlockchain {
    type Error;

    fn last_block(&self) -> rethnet_eth::block::Block {
        todo!()
    }

    fn insert_block(&mut self, block: rethnet_eth::block::Block) -> Result<(), Self::Error> {
        todo!()
    }
}
