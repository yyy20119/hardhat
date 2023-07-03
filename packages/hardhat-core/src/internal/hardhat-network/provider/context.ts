import { Common } from "@nomicfoundation/ethereumjs-common";
import { VMAdapter } from "./vm/vm-adapter";
import { MemPoolAdapter } from "./mem-pool";
import { BlockMinerAdapter } from "./miner";
import { BlockBuilderAdapter, BuildBlockOpts } from "./vm/block-builder";
import { HardhatBlockchainInterface } from "./types/HardhatBlockchainInterface";

export interface EthContextAdapter {
  blockchain(): HardhatBlockchainInterface;

  blockBuilder(
    common: Common,
    opts: BuildBlockOpts
  ): Promise<BlockBuilderAdapter>;

  blockMiner(): BlockMinerAdapter;

  memPool(): MemPoolAdapter;

  vm(): VMAdapter;
}
