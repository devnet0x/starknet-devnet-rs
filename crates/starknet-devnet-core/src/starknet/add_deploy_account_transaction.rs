use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
use starknet_types::rpc::transactions::deploy_account_transaction_v3::DeployAccountTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedDeployAccountTransaction, DeployAccountTransaction, Transaction,
};

use super::dump::DumpEvent;
use super::Starknet;
use crate::error::{DevnetResult, Error};
use crate::traits::StateExtractor;

pub fn add_deploy_account_transaction_v3(
    starknet: &mut Starknet,
    broadcasted_deploy_account_transaction: BroadcastedDeployAccountTransactionV3,
) -> DevnetResult<(TransactionHash, ContractAddress)> {
    if broadcasted_deploy_account_transaction.common.is_max_fee_zero_value() {
        return Err(Error::MaxFeeZeroError { tx_type: "deploy account transaction v3".into() });
    }

    if !starknet.state.is_contract_declared(&broadcasted_deploy_account_transaction.class_hash) {
        return Err(Error::StateError(crate::error::StateError::NoneClassHash(
            broadcasted_deploy_account_transaction.class_hash,
        )));
    }

    let blockifier_deploy_account_transaction = broadcasted_deploy_account_transaction
        .create_blockifier_deploy_account(starknet.chain_id().to_felt(), false)?;

    let transaction_hash = blockifier_deploy_account_transaction.tx_hash.0.into();
    let address: ContractAddress = blockifier_deploy_account_transaction.contract_address.into();
    let deploy_account_transaction_v3 = DeployAccountTransactionV3::new(
        broadcasted_deploy_account_transaction.clone(),
        address,
        transaction_hash,
    );

    let transaction = Transaction::DeployAccount(DeployAccountTransaction::Version3(Box::new(
        deploy_account_transaction_v3,
    )));

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::DeployAccount(
            blockifier_deploy_account_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(transaction, None, blockifier_execution_result)?;
    starknet.handle_dump_event(DumpEvent::AddDeployAccountTransaction(
        BroadcastedDeployAccountTransaction::V3(broadcasted_deploy_account_transaction),
    ))?;

    Ok((transaction_hash, address))
}

pub fn add_deploy_account_transaction_v1(
    starknet: &mut Starknet,
    broadcasted_deploy_account_transaction: BroadcastedDeployAccountTransactionV1,
) -> DevnetResult<(TransactionHash, ContractAddress)> {
    if broadcasted_deploy_account_transaction.common.max_fee.0 == 0 {
        return Err(Error::MaxFeeZeroError { tx_type: "deploy account transaction".into() });
    }

    if !starknet.state.is_contract_declared(&broadcasted_deploy_account_transaction.class_hash) {
        return Err(Error::StateError(crate::error::StateError::NoneClassHash(
            broadcasted_deploy_account_transaction.class_hash,
        )));
    }

    let blockifier_deploy_account_transaction = broadcasted_deploy_account_transaction
        .create_blockifier_deploy_account(starknet.chain_id().to_felt(), false)?;

    let transaction_hash = blockifier_deploy_account_transaction.tx_hash.0.into();
    let address: ContractAddress = blockifier_deploy_account_transaction.contract_address.into();
    let deploy_account_transaction_v1 = broadcasted_deploy_account_transaction
        .compile_deploy_account_transaction_v1(&transaction_hash, address);

    let transaction = Transaction::DeployAccount(DeployAccountTransaction::Version1(Box::new(
        deploy_account_transaction_v1,
    )));

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::DeployAccount(
            blockifier_deploy_account_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(transaction, None, blockifier_execution_result)?;
    starknet.handle_dump_event(DumpEvent::AddDeployAccountTransaction(
        BroadcastedDeployAccountTransaction::V1(broadcasted_deploy_account_transaction),
    ))?;

    Ok((transaction_hash, address))
}

#[cfg(test)]
mod tests {

    use starknet_api::transaction::{Fee, Tip};
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::Cairo0Json;
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::{ClassHash, Felt};
    use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
    use starknet_types::rpc::transactions::{
        BroadcastedTransactionCommonV3, ResourceBoundsWrapper,
    };
    use starknet_types::traits::HashProducer;

    use crate::constants::{
        self, DEVNET_DEFAULT_CHAIN_ID, ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
    };
    use crate::error::Error;
    use crate::starknet::{predeployed, Starknet};
    use crate::traits::{Deployed, HashIdentifiedMut, StateChanger, StateExtractor};
    use crate::utils::get_storage_var_address;

    fn test_deploy_account_transaction_v3(
        class_hash: ClassHash,
        nonce: u128,
        l1_gas_amount: u64,
    ) -> BroadcastedDeployAccountTransactionV3 {
        BroadcastedDeployAccountTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::from(3),
                signature: vec![],
                nonce: Felt::from(nonce),
                resource_bounds: ResourceBoundsWrapper::new(l1_gas_amount, 1, 0, 0),
                tip: Tip(0),
                paymaster_data: vec![],
                nonce_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
                fee_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
            },
            contract_address_salt: 0.into(),
            constructor_calldata: vec![],
            class_hash,
        }
    }

    #[test]
    fn account_deploy_transaction_v1_with_max_fee_zero_should_return_an_error() {
        let deploy_account_transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![0.into(), 1.into()],
            Fee(0),
            &vec![0.into(), 1.into()],
            0.into(),
            0.into(),
            0.into(),
            0.into(),
        );

        let result =
            Starknet::default().add_deploy_account_transaction_v1(deploy_account_transaction);

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "deploy account transaction: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn deploy_account_transaction_v3_with_max_fee_zero_should_return_an_error() {
        let (mut starknet, account_class_hash, _, _) = setup();
        let deploy_account_transaction =
            test_deploy_account_transaction_v3(account_class_hash, 0, 0);

        let txn_err =
            starknet.add_deploy_account_transaction_v3(deploy_account_transaction).unwrap_err();
        match txn_err {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "deploy account transaction v3: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn deploy_account_transaction_v1_should_return_an_error_due_to_not_enough_balance() {
        let (mut starknet, account_class_hash, _, _) = setup();

        let fee_raw: u128 = 4000;
        let transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![],
            Fee(fee_raw),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );

        match starknet.add_deploy_account_transaction_v1(transaction).unwrap_err() {
            Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    #[test]
    fn deploy_account_transaction_v3_should_return_an_error_due_to_not_enough_balance() {
        let (mut starknet, account_class_hash, _, _) = setup();
        let transaction = test_deploy_account_transaction_v3(account_class_hash, 0, 4000);

        match starknet.add_deploy_account_transaction_v3(transaction).unwrap_err() {
            Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    #[test]
    fn deploy_account_transaction_v1_should_return_an_error_due_to_not_enough_fee() {
        let (mut starknet, account_class_hash, eth_fee_token_address, _) = setup();

        let fee_raw: u128 = 2000;
        let transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![],
            Fee(fee_raw),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );

        let blockifier_transaction = transaction
            .create_blockifier_deploy_account(DEVNET_DEFAULT_CHAIN_ID.to_felt(), false)
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(blockifier_transaction.contract_address);
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()]).unwrap();
        let balance_storage_key =
            ContractStorageKey::new(eth_fee_token_address, balance_storage_var_address);

        starknet.state.change_storage(balance_storage_key, Felt::from(fee_raw)).unwrap();
        starknet.state.clear_dirty_state();

        match starknet.add_deploy_account_transaction_v1(transaction).unwrap_err() {
            Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientMaxFee,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    #[test]
    fn test_deploy_account_transaction_v3_successful_execution() {
        let (mut starknet, account_class_hash, _, strk_fee_token_address) = setup();
        let transaction = test_deploy_account_transaction_v3(account_class_hash, 0, 4000);

        let blockifier_transaction = transaction
            .create_blockifier_deploy_account(DEVNET_DEFAULT_CHAIN_ID.to_felt(), false)
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(blockifier_transaction.contract_address);
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()]).unwrap();
        let balance_storage_key =
            ContractStorageKey::new(strk_fee_token_address, balance_storage_var_address);

        let account_balance_before_deployment = Felt::from(1000000);
        starknet
            .state
            .change_storage(balance_storage_key, account_balance_before_deployment)
            .unwrap();
        starknet.state.clear_dirty_state();

        // get accounts count before deployment
        let accounts_before_deployment = get_accounts_count(&starknet);

        let (txn_hash, _) = starknet.add_deploy_account_transaction_v3(transaction).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(txn.execution_result.status(), TransactionExecutionStatus::Succeeded);

        assert_eq!(get_accounts_count(&starknet), accounts_before_deployment + 1);
        let account_balance_after_deployment =
            starknet.state.get_storage(balance_storage_key).unwrap();

        assert!(account_balance_before_deployment > account_balance_after_deployment);
    }

    fn get_accounts_count(starknet: &Starknet) -> usize {
        starknet.state.state.state.address_to_class_hash.len()
    }

    #[test]
    fn deploy_account_transaction_v1_successful_execution() {
        let (mut starknet, account_class_hash, eth_fee_token_address, _) = setup();

        let transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![],
            Fee(4000),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );
        let blockifier_transaction = transaction
            .create_blockifier_deploy_account(DEVNET_DEFAULT_CHAIN_ID.to_felt(), false)
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(blockifier_transaction.contract_address);
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()]).unwrap();
        let balance_storage_key =
            ContractStorageKey::new(eth_fee_token_address, balance_storage_var_address);

        let account_balance_before_deployment = Felt::from(1000000);
        starknet
            .state
            .change_storage(balance_storage_key, account_balance_before_deployment)
            .unwrap();
        starknet.state.clear_dirty_state();

        // get accounts count before deployment
        let accounts_before_deployment = get_accounts_count(&starknet);

        let (txn_hash, _) = starknet.add_deploy_account_transaction_v1(transaction).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(txn.execution_result.status(), TransactionExecutionStatus::Succeeded);

        assert_eq!(get_accounts_count(&starknet), accounts_before_deployment + 1);
        let account_balance_after_deployment =
            starknet.state.get_storage(balance_storage_key).unwrap();

        assert!(account_balance_before_deployment > account_balance_after_deployment);
    }

    /// Initializes starknet with erc20 contract, 1 declared contract class. Gas price is set to 1
    fn setup() -> (Starknet, ClassHash, ContractAddress, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let erc_20_contract =
            predeployed::create_erc20_at_address(ETH_ERC20_CONTRACT_ADDRESS).unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let strk_erc20_contract =
            predeployed::create_erc20_at_address(STRK_ERC20_CONTRACT_ADDRESS).unwrap();
        strk_erc20_contract.deploy(&mut starknet.state).unwrap();

        let contract_class = Cairo0Json::raw_json_from_path(account_json_path).unwrap();
        let class_hash = contract_class.generate_hash().unwrap();

        starknet.state.declare_contract_class(class_hash, contract_class.into()).unwrap();
        starknet.state.clear_dirty_state();
        starknet.block_context = Starknet::init_block_context(
            1,
            constants::ETH_ERC20_CONTRACT_ADDRESS,
            constants::STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        );

        starknet.restart_pending_block().unwrap();

        (starknet, class_hash, erc_20_contract.get_address(), strk_erc20_contract.get_address())
    }
}
