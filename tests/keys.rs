use dewiz_stealth_address::stealth;

#[tokio::test]
async fn check_key_generation() {

    tracing_subscriber::fmt::try_init().expect("Tracing setup must be correctly set.");

    tracing::info!("=== ERC-5564 Stealth Address Demo (secp256k1, Scheme ID 1) ===\n");

    // -----------------------------------------------------------------------
    // Step 1: Bob generates stealth meta-address
    // -----------------------------------------------------------------------
    tracing::info!("--- Step 1: Bob generates stealth meta-address ---");
    let meta = stealth::generate_meta_address();

    tracing::info!(
        "  spending private key (k): {}",
        stealth::scalar_to_hex(&meta.spending_key)
    );
    tracing::info!(
        "  spending public key  (K): {}",
        stealth::point_to_hex(&meta.spending_pubkey)
    );
    tracing::info!(
        "  viewing private key  (v): {}",
        stealth::scalar_to_hex(&meta.viewing_key)
    );
    tracing::info!(
        "  viewing public key   (V): {}",
        stealth::point_to_hex(&meta.viewing_pubkey)
    );
    tracing::info!("  meta-address: {}", stealth::format_meta_address(&meta));

    // -----------------------------------------------------------------------
    // Step 2: Alice parses Bob's meta-address and generates stealth address
    // -----------------------------------------------------------------------
    tracing::info!("\n--- Step 2: Alice parses Bob's meta-address and generates stealth address ---");

    // Alice only has the meta-address string (e.g. shared via ENS, QR code, etc.)
    let meta_address_str = stealth::format_meta_address(&meta);
    let (parsed_k, parsed_v) =
        stealth::parse_meta_address(&meta_address_str).expect("meta-address should be valid");
    tracing::info!("  parsed spending pubkey (K): {}", stealth::point_to_hex(&parsed_k));
    tracing::info!("  parsed viewing pubkey  (V): {}", stealth::point_to_hex(&parsed_v));

    let output = stealth::generate_stealth_address(&parsed_k, &parsed_v);

    tracing::info!(
        "  ephemeral public key (R): {}",
        stealth::point_to_hex(&output.ephemeral_pubkey)
    );
    tracing::info!(
        "  stealth public key   (P): {}",
        stealth::point_to_hex(&output.stealth_pubkey)
    );
    tracing::info!(
        "  stealth address:          {}",
        stealth::addr_to_hex(&output.stealth_address)
    );
    tracing::info!("  view tag:                 0x{:02x}", output.view_tag);

    tracing::info!(
        "\n  Alice sends 100 USDS to {}",
        stealth::addr_to_hex(&output.stealth_address)
    );
    tracing::info!(
        "  Alice publishes announcement: (R, tag=0x{:02x})",
        output.view_tag
    );

    // -----------------------------------------------------------------------
    // Step 3: Bob scans announcement and recovers stealth private key
    // -----------------------------------------------------------------------
    tracing::info!("\n--- Step 3: Bob scans announcement and recovers stealth private key ---");
    let recovered = stealth::scan_and_recover(
        &meta,
        &output.ephemeral_pubkey,
        output.view_tag,
        &output.stealth_address,
    )
    .expect("Bob should find the payment");

    tracing::info!("  view tag matched:     yes");
    tracing::info!("  address matched:      yes");
    tracing::info!(
        "  stealth private key (p): {}",
        stealth::scalar_to_hex(&recovered.stealth_private_key)
    );
    tracing::info!(
        "  recovered address:       {}",
        stealth::addr_to_hex(&recovered.stealth_address)
    );

    // -----------------------------------------------------------------------
    // Step 4: Verification
    // -----------------------------------------------------------------------
    tracing::info!("\n--- Step 4: Verification ---");
    let valid = stealth::verify(&recovered.stealth_private_key, &output.stealth_pubkey);
    assert!(valid, "Verification failed: p·G ≠ P");
    tracing::info!("  p·G == P: {} (protocol correctness verified)", valid);

    // -----------------------------------------------------------------------
    // Step 5: Demonstrate parse_pubkey_hex with the ephemeral key
    // -----------------------------------------------------------------------
    tracing::info!("\n--- Step 5: parse_pubkey_hex demo ---");
    let r_hex = stealth::point_to_hex(&output.ephemeral_pubkey);
    let r_parsed = stealth::parse_pubkey_hex(&r_hex).expect("ephemeral pubkey hex should be valid");
    tracing::info!("  original R:  {}", r_hex);
    tracing::info!("  parsed   R:  {}", stealth::point_to_hex(&r_parsed));
    assert_eq!(
        stealth::point_to_hex(&output.ephemeral_pubkey),
        stealth::point_to_hex(&r_parsed),
        "round-trip parse should match"
    );
    tracing::info!("  round-trip: OK");

    tracing::info!(
        "\n=== Done. Bob can now sign transactions from {} ===",
        stealth::addr_to_hex(&output.stealth_address)
    );
}