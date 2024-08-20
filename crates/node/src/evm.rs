//! # AlphaNet EVM configuration
//!
//! The [AlphaNetEvmConfig] type implements the [ConfigureEvm] and [ConfigureEvmEnv] traits,
//! configuring the custom AlphaNet precompiles and instructions.
//!
//! These trait implementations allow for custom precompiles and instructions to be implemented and
//! integrated in a reth node only with importing, without the need to fork the node or EVM
//! implementation.
//!
//! This currently configures the instructions defined in [EIP3074-instructions](https://github.com/paradigmxyz/eip3074-instructions), and the
//! precompiles defined by [`alphanet_precompile`].

use alphanet_precompile::{bls12_381, secp256r1};
use reth::{
    primitives::{
        revm_primitives::{CfgEnvWithHandlerCfg, TxEnv},
        Address, Bytes, Header, TransactionSigned, U256,
    },
    revm::{
        handler::register::EvmHandler,
        inspector_handle_register,
        precompile::PrecompileSpecId,
        primitives::{Env, SpecId},
        ContextPrecompiles, Database, Evm, EvmBuilder, GetInspector,
    },
};
use reth_chainspec::ChainSpec;
use reth_node_api::{ConfigureEvm, ConfigureEvmEnv};
use reth_node_optimism::OptimismEvmConfig;
use std::sync::Arc;

/// Custom EVM configuration
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct AlphaNetEvmConfig;

impl AlphaNetEvmConfig {
    /// Sets the precompiles to the EVM handler
    ///
    /// This will be invoked when the EVM is created via [ConfigureEvm::evm] or
    /// [ConfigureEvm::evm_with_inspector]
    ///
    /// This will use the default mainnet precompiles and add additional precompiles.
    fn set_precompiles<EXT, DB>(handler: &mut EvmHandler<'_, EXT, DB>)
    where
        DB: Database,
    {
        // first we need the evm spec id, which determines the precompiles
        let spec_id = handler.cfg.spec_id;

        // install the precompiles
        handler.pre_execution.load_precompiles = Arc::new(move || {
            let mut loaded_precompiles: ContextPrecompiles<DB> =
                ContextPrecompiles::new(PrecompileSpecId::from_spec_id(spec_id));

            loaded_precompiles.extend(secp256r1::precompiles());
            loaded_precompiles.extend(bls12_381::precompiles());

            loaded_precompiles
        });
    }
}

impl ConfigureEvm for AlphaNetEvmConfig {
    type DefaultExternalContext<'a> = ();

    fn evm<DB: Database>(&self, db: DB) -> Evm<'_, Self::DefaultExternalContext<'_>, DB> {
        EvmBuilder::default()
            .with_db(db)
            .optimism()
            // add additional precompiles
            .append_handler_register(Self::set_precompiles)
            .build()
    }

    fn evm_with_inspector<DB, I>(&self, db: DB, inspector: I) -> Evm<'_, I, DB>
    where
        DB: Database,
        I: GetInspector<DB>,
    {
        EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            .optimism()
            // add additional precompiles
            .append_handler_register(Self::set_precompiles)
            .append_handler_register(inspector_handle_register)
            .build()
    }

    fn default_external_context<'a>(&self) -> Self::DefaultExternalContext<'a> {}
}

impl ConfigureEvmEnv for AlphaNetEvmConfig {
    fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &TransactionSigned, sender: Address) {
        OptimismEvmConfig::default().fill_tx_env(tx_env, transaction, sender)
    }

    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        chain_spec: &ChainSpec,
        header: &Header,
        total_difficulty: U256,
    ) {
        OptimismEvmConfig::default().fill_cfg_env(cfg_env, chain_spec, header, total_difficulty);

        // TODO(onbjerg): Remove this once Prague and PragueEOF are merged into one.
        // Map Prague to PragueEOF to enable EOF support on Alphanet.
        if cfg_env.handler_cfg.spec_id == SpecId::PRAGUE {
            cfg_env.handler_cfg.spec_id = SpecId::PRAGUE_EOF;
        }
    }

    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        OptimismEvmConfig::default().fill_tx_env_system_contract_call(env, caller, contract, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reth::primitives::{
        revm_primitives::{BlockEnv, CfgEnv, SpecId},
        ForkCondition, Genesis,
    };
    use reth_chainspec::{Chain, ChainSpecBuilder, EthereumHardfork};

    #[test]
    fn test_fill_cfg_and_block_env() {
        let mut cfg_env = CfgEnvWithHandlerCfg::new_with_spec_id(CfgEnv::default(), SpecId::LATEST);
        let mut block_env = BlockEnv::default();
        let header = Header::default();
        let chain_spec = ChainSpecBuilder::default()
            .chain(Chain::optimism_mainnet())
            .genesis(Genesis::default())
            .with_fork(EthereumHardfork::Frontier, ForkCondition::Block(0))
            .build();
        let total_difficulty = U256::ZERO;

        AlphaNetEvmConfig::default().fill_cfg_and_block_env(
            &mut cfg_env,
            &mut block_env,
            &chain_spec,
            &header,
            total_difficulty,
        );

        assert_eq!(cfg_env.chain_id, chain_spec.chain().id());
    }
}
