mod result;

use std::{ops::Deref, sync::Arc};

use napi::{
    bindgen_prelude::{BigInt, Buffer},
    tokio::sync::RwLock,
    Status,
};
use napi_derive::napi;
use rethnet_eth::{Address, U256};
use rethnet_evm::{blockchain::BlockchainError, state::StateError, RandomHashGenerator};

use crate::{
    blockchain::Blockchain,
    cast::TryCast,
    config::Config,
    context::{Context, RethnetContext},
    mempool::MemPool,
    state::StateManager,
};

use self::result::MineBlockResult;

#[napi]
pub struct BlockMiner {
    miner: Arc<RwLock<rethnet_evm::BlockMiner<BlockchainError, StateError>>>,
    context: Arc<Context>,
}

impl Deref for BlockMiner {
    type Target = Arc<RwLock<rethnet_evm::BlockMiner<BlockchainError, StateError>>>;

    fn deref(&self) -> &Self::Target {
        &self.miner
    }
}

#[napi]
impl BlockMiner {
    #[doc = "Constructs a new [`BlockMiner`]."]
    #[napi(constructor)]
    pub fn new(
        context: &RethnetContext,
        blockchain: &Blockchain,
        state_manager: &StateManager,
        mem_pool: &MemPool,
        cfg: &Config,
        block_gas_limit: BigInt,
        beneficiary: Buffer,
    ) -> napi::Result<Self> {
        let context = (*context).clone();
        let blockchain = (*blockchain).clone();
        let state = (*state_manager).clone();
        let mem_pool = (*mem_pool).clone();
        let cfg = (*cfg).clone();
        let block_gas_limit: U256 = BigInt::try_cast(block_gas_limit)?;
        let beneficiary = Address::from_slice(&beneficiary);

        let prevrandao_generator = RandomHashGenerator::with_seed("randomMixHashSeed");
        let miner = rethnet_evm::BlockMiner::new(
            blockchain,
            state,
            mem_pool,
            prevrandao_generator,
            cfg,
            block_gas_limit,
            beneficiary,
        );

        Ok(Self {
            miner: Arc::new(RwLock::new(miner)),
            context,
        })
    }

    #[napi]
    pub async fn mine_block(
        &self,
        timestamp: BigInt,
        reward: BigInt,
        base_fee: Option<BigInt>,
    ) -> napi::Result<MineBlockResult> {
        let timestamp: U256 = BigInt::try_cast(timestamp)?;
        let reward: U256 = BigInt::try_cast(reward)?;
        let base_fee: Option<U256> =
            base_fee.map_or(Ok(None), |base_fee| BigInt::try_cast(base_fee).map(Some))?;

        let miner = self.miner.clone();

        self.context
            .runtime()
            .spawn(async move {
                let mut miner = miner.write().await;
                miner.mine_block(timestamp, reward, base_fee).await
            })
            .await
            .unwrap()
            .map_or_else(
                |e| Err(napi::Error::new(Status::GenericFailure, e.to_string())),
                |result| Ok(MineBlockResult::from(result)),
            )
    }
}
