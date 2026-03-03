mod common;

use alloy::{
    network::TransactionBuilder,
    primitives::{
        U256,
        utils::parse_units,
    },
    providers::{
        Provider,
        WalletProvider,
    },
    rpc::types::TransactionRequest,
};
use std::time::Duration;
use dewiz_stealth_address::stealth;
use common::TestApp;

#[tokio::test]
async fn send_stealth_transaction() {
    let app = TestApp::new();
    
    let sender_address = app.client_sender.wallet().default_signer().address();
    tracing::info!("Sender address: {}", sender_address);

    // Based on Rune's spending key and viewing key, a new stealth output for him is generated.
    // He might or not be aware of this new stealth address, 
    // but it will be generated based on his meta-address.
    let rune_new_stealth_output = stealth::generate_stealth_address(
        &app.stealth_key_receiver.spending_pubkey,
        &app.stealth_key_receiver.viewing_pubkey,
    );
    
    let rune_new_address = rune_new_stealth_output.to_ethereum_address();

    tracing::info!(
        "Generated stealth address for Rune: {}",
        rune_new_address
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

        tracing::info!("Transferring 0.1 USDC to Rune's new stealth address: {:?}...\n\n", rune_new_address);
        let tx_receipt = app.erc20_attached_to_sender_wallet
            .transfer(rune_new_address, test_transfer_token_value)
            .max_priority_fee_per_gas(increased_priority_fee)
            .send().await.expect("Failed to send transaction")
                .with_required_confirmations(1)
                .with_timeout(Some(Duration::from_secs(120)))
                .get_receipt().await.expect("Failed to get transaction receipt");
        
        tracing::info!("Transfer of 0.1 USDC to {:?} completed successfully!\n\n", rune_new_address);
        tracing::info!("Transaction details: {:?}\n\n", tx_receipt);

        // Build a transaction to send 21000*100 wei from Sender to Rune's new stealth address.
        // So it can execute the withdrawal from the stealth address and pay for the gas fees.
        let test_transfer_eth_value: U256 = parse_units("0.00001", 18 as u8).expect("invalid units").into();
        let tx =
            TransactionRequest::default()
            .with_from(sender_address)
            .with_to(rune_new_address)
            .with_value(test_transfer_eth_value)
            .with_max_priority_fee_per_gas(increased_priority_fee);

        // // Send the transaction and listen for the transaction to be included.
        tracing::info!("Sending {} wei transaction to Rune's new stealth address: {:?}...\n\n", test_transfer_eth_value, rune_new_address);
        let tx_hash = app.client_sender
        .send_transaction(tx).await.expect("Fail to send transaction")
        .watch().await.expect("Fail to process the transaction");

        tracing::info!("Transaction processed with hash: {:?}\n\n", tx_hash);

    }


}