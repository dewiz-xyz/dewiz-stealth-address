#[path = "helpers.rs"]
mod helpers;

use alloy::primitives::Address;
use helpers::TestApp;

#[tokio::test]
async fn test_usdc_correct_address() {
    let app = TestApp::new();
    let app_usdc_address = app.erc20_destination_contract_address;
    let expected_usdc_address: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        .to_owned()
        .parse()
        .expect("Invalid contract address");
    assert_eq!(
        app_usdc_address, expected_usdc_address,
        "USDC contract address does not match expected value"
    );
    tracing::info!("USDC contract address: 0x{:x}", app_usdc_address);
}