mod common;

use alloy::{
    network::TransactionBuilder,
    primitives::{
        U256,
        utils::parse_units,
        Address,
    },
    providers::{
        Provider, ProviderBuilder, WalletProvider
    },
    rpc::types::TransactionRequest,
};
use std::time::Duration;
use dewiz_stealth_address::stealth;
use common::TestApp;

use crate::common::smartcontract::abi::ERC20::ERC20Instance;

#[tokio::test]
async fn send_stealth_transaction() {
    let app = TestApp::new();
    
    let sender_address = app.client_sender.wallet().default_signer().address();
    tracing::info!("Sender address: {}", sender_address);

    // Based on Bob's spending key and viewing key, a new stealth output for him is generated.
    // He might or not be aware of this new stealth address, 
    // but it will be generated based on his meta-address.
    let bob_new_stealth_output = stealth::generate_stealth_address(
        &app.stealth_key_receiver.spending_pubkey,
        &app.stealth_key_receiver.viewing_pubkey,
    );
    
    let bob_new_address = bob_new_stealth_output.to_ethereum_address();

    tracing::info!(
        "Generated stealth address for Bob: {}",
        bob_new_address
    );

    let bob_new_wallet_keys = stealth::scan_and_recover(
        &app.stealth_key_receiver,
        &bob_new_stealth_output.ephemeral_pubkey,
        bob_new_stealth_output.view_tag,
        &bob_new_stealth_output.stealth_address,
    );
    assert!(bob_new_wallet_keys.is_some(), "Bob should detect the payment");
    let bob_new_wallet_keys = bob_new_wallet_keys.unwrap();
    assert_eq!(bob_new_wallet_keys.stealth_address, bob_new_stealth_output.stealth_address);
    assert!(stealth::verify(
        &bob_new_wallet_keys.stealth_private_key,
        &bob_new_stealth_output.stealth_pubkey
    ));
    let bob_new_wallet = bob_new_wallet_keys.to_wallet();
    tracing::info!(
        "Bob successfully recovered the stealth private key for the new stealth address and wallet: {} - {:?}",
        bob_new_wallet_keys.to_ethereum_address(),
        bob_new_wallet
    );

    let bob_new_provider = ProviderBuilder::new()
        .wallet(bob_new_wallet)
        .connect_http(app.rpc_url.clone());
    
    let bob_new_erc20_instance = ERC20Instance::new(
        app.erc20_destination_contract_address, 
        bob_new_provider.clone()
    );

    tracing::info!(
        "Bob's new stealth wallet balance in USDC: {:?}",
        bob_new_erc20_instance.balanceOf(
            bob_new_wallet_keys.to_ethereum_address()
        ).call().await.expect("Failed to fetch new Bob's account balance")
    );
    
    let test_transfer = true;

    if test_transfer {
        // Fetch current suggested priority fee and increase it for faster inclusion
        let suggested_priority_fee = app.client_sender
            .get_max_priority_fee_per_gas().await
            .expect("Failed to fetch max priority fee per gas");
        let increased_priority_fee = suggested_priority_fee * 5;
        tracing::info!("Suggested priority fee: {} wei, using increased: {} wei", suggested_priority_fee, increased_priority_fee);

        let test_transfer_token_value: U256 = parse_units("0.1", 6 as u8).expect("invalid units").into();

        tracing::info!("Transferring 0.1 USDC to Bob's new stealth address: {:?}...\n\n", bob_new_address);
        let mut tx_receipt = app.erc20_attached_to_sender_wallet
            .transfer(bob_new_address, test_transfer_token_value)
            .max_priority_fee_per_gas(increased_priority_fee)
            .send().await.expect("Failed to send transaction")
                .with_required_confirmations(1)
                .with_timeout(Some(Duration::from_secs(120)))
                .get_receipt().await.expect("Failed to get transaction receipt");
        
        tracing::info!("Transfer of 0.1 USDC to {:?} completed successfully!\n\n", bob_new_address);
        tracing::info!("Transaction details: {:?}\n\n", tx_receipt);

        // Build a transaction to send wei from Sender to Bob's new stealth address.
        // So it can execute the withdrawal from the stealth address and pay for the gas fees.
        let test_transfer_eth_value: U256 = parse_units("0.00009", 18 as u8).expect("invalid units").into();
        let tx =
            TransactionRequest::default()
            .with_from(sender_address)
            .with_to(bob_new_address)
            .with_value(test_transfer_eth_value)
            .with_max_priority_fee_per_gas(increased_priority_fee);

        // // Send the transaction and listen for the transaction to be included.
        tracing::info!("Sending {} wei transaction to Bob's new stealth address: {:?}...\n", test_transfer_eth_value, bob_new_address);
        let tx_hash = app.client_sender
        .send_transaction(tx).await.expect("Fail to send transaction")
        .watch().await.expect("Fail to process the transaction");

        tracing::info!("Transaction processed with hash: {:?}\n", tx_hash);
        // tracing::info!("###############################################\n\n");

        tracing::info!(
            "Bob's new stealth wallet balance in USDC after transfer: {:?}\n",
            bob_new_erc20_instance.balanceOf(
                bob_new_wallet_keys.to_ethereum_address()
            ).call().await.expect("Failed to fetch new Bob's account balance after transfer")
        );
        tracing::info!(
            "Bob's new stealth wallet balance in ETH after transfer: {:?}\n",
            bob_new_provider.get_balance(bob_new_wallet_keys.to_ethereum_address()).await.expect("Failed to fetch new Bob's account balance in ETH after transfer")
        );

        tracing::info!(
            "Bob will sends back the funds to the a thrid-party address to test 
            the stealth wallet functionality for outgoing transactions...\n"
        );

        let third_party_address: Address = "0x1dA2547202458D2540d64513D409A1c2bA57bA3B".parse().expect("Invalid third-party address");

        tx_receipt = bob_new_erc20_instance
            .transfer(third_party_address, test_transfer_token_value)
            .max_priority_fee_per_gas(increased_priority_fee)
            .send().await.expect("Failed to send transaction to third-party address")
                .with_required_confirmations(1)
                .with_timeout(Some(Duration::from_secs(120)))
                .get_receipt().await.expect("Failed to get transaction receipt for transfer to third-party address");
        
        tracing::info!(
            "Transfer of 0.1 USDC to {:?} the third-party address completed successfully!\n\n", 
            third_party_address
        );
        tracing::info!("Transaction details of transfer to third-party address: {:?}\n\n", tx_receipt);

        tracing::info!(
            "Bob's new stealth wallet balance in USDC after second transfer: {:?}\n",
            bob_new_erc20_instance.balanceOf(
                bob_new_wallet_keys.to_ethereum_address()
            ).call().await.expect("Failed to fetch new Bob's account balance after second transfer")
        );
        tracing::info!(
            "Bob's new stealth wallet balance in ETH after second transfer: {:?}\n",
            bob_new_provider.get_balance(
                bob_new_wallet_keys.to_ethereum_address()
            ).await.expect("Failed to fetch new Bob's account balance in ETH after second transfer")
        );
    }


}