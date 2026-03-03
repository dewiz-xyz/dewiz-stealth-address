mod common;

use alloy::primitives::Address;

use common::TestApp;

#[tokio::test]
async fn test_sender_and_receiver_addresses_are_set() {
    let app = TestApp::new();

    let receiver_address = app.get_receiver_address().await;
    let sender_address = app.get_sender_address().await;
    tracing::info!("Receiver address: 0x{:x}", receiver_address);
    tracing::info!("Sender address: 0x{:x}", sender_address);
    assert_ne!(
        receiver_address, sender_address,
        "Receiver and sender addresses should not be the same"
    );
    assert_ne!(
        receiver_address, Address::ZERO,
        "Receiver address should not be the zero address"
    );
    assert_ne!(
        sender_address, Address::ZERO,
        "Sender address should not be the zero address"
    );
}

