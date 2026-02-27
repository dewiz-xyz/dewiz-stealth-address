use k256::{
    elliptic_curve::sec1::ToEncodedPoint,
    ProjectivePoint, 
    Scalar,
};

use dewiz_stealth_address::stealth;

fn point_hex(p: &ProjectivePoint) -> String {
    hex::encode(p.to_affine().to_encoded_point(true).as_bytes())
}

fn scalar_hex(s: &Scalar) -> String {
    hex::encode(s.to_bytes())
}

fn addr_hex(a: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(a))
}

fn main() {
    println!("=== ERC-5564 Stealth Address Demo (secp256k1, Scheme ID 1) ===\n");

    // -----------------------------------------------------------------------
    // Step 1: Bob generates stealth meta-address
    // -----------------------------------------------------------------------
    println!("--- Step 1: Bob generates stealth meta-address ---");
    let meta = stealth::generate_meta_address();

    println!("  spending private key (k): {}", scalar_hex(&meta.spending_key));
    println!("  spending public key  (K): {}", point_hex(&meta.spending_pubkey));
    println!("  viewing private key  (v): {}", scalar_hex(&meta.viewing_key));
    println!("  viewing public key   (V): {}", point_hex(&meta.viewing_pubkey));
    println!("  meta-address: {}", stealth::format_meta_address(&meta));

    // -----------------------------------------------------------------------
    // Step 2: Alice parses Bob's meta-address and generates stealth address
    // -----------------------------------------------------------------------
    println!("\n--- Step 2: Alice parses Bob's meta-address and generates stealth address ---");

    // Alice only has the meta-address string (e.g. shared via ENS, QR code, etc.)
    let meta_address_str = stealth::format_meta_address(&meta);
    let (parsed_k, parsed_v) = stealth::parse_meta_address(&meta_address_str)
        .expect("meta-address should be valid");
    println!("  parsed spending pubkey (K): {}", point_hex(&parsed_k));
    println!("  parsed viewing pubkey  (V): {}", point_hex(&parsed_v));

    let output = stealth::generate_stealth_address(&parsed_k, &parsed_v);

    println!("  ephemeral public key (R): {}", point_hex(&output.ephemeral_pubkey));
    println!("  stealth public key   (P): {}", point_hex(&output.stealth_pubkey));
    println!("  stealth address:          {}", addr_hex(&output.stealth_address));
    println!("  view tag:                 0x{:02x}", output.view_tag);

    println!("\n  Alice sends 100 USDS to {}", addr_hex(&output.stealth_address));
    println!("  Alice publishes announcement: (R, tag=0x{:02x})", output.view_tag);

    // -----------------------------------------------------------------------
    // Step 3: Bob scans announcement and recovers stealth private key
    // -----------------------------------------------------------------------
    println!("\n--- Step 3: Bob scans announcement and recovers stealth private key ---");
    let recovered = stealth::scan_and_recover(
        &meta,
        &output.ephemeral_pubkey,
        output.view_tag,
        &output.stealth_address,
    )
    .expect("Bob should find the payment");

    println!("  view tag matched:     yes");
    println!("  address matched:      yes");
    println!("  stealth private key (p): {}", scalar_hex(&recovered.stealth_private_key));
    println!("  recovered address:       {}", addr_hex(&recovered.stealth_address));

    // -----------------------------------------------------------------------
    // Step 4: Verification
    // -----------------------------------------------------------------------
    println!("\n--- Step 4: Verification ---");
    let valid = stealth::verify(&recovered.stealth_private_key, &output.stealth_pubkey);
    assert!(valid, "Verification failed: p·G ≠ P");
    println!("  p·G == P: {} (protocol correctness verified)", valid);

    // -----------------------------------------------------------------------
    // Step 5: Demonstrate parse_pubkey_hex with the ephemeral key
    // -----------------------------------------------------------------------
    println!("\n--- Step 5: parse_pubkey_hex demo ---");
    let r_hex = point_hex(&output.ephemeral_pubkey);
    let r_parsed = stealth::parse_pubkey_hex(&r_hex)
        .expect("ephemeral pubkey hex should be valid");
    println!("  original R:  {}", r_hex);
    println!("  parsed   R:  {}", point_hex(&r_parsed));
    assert_eq!(
        point_hex(&output.ephemeral_pubkey),
        point_hex(&r_parsed),
        "round-trip parse should match"
    );
    println!("  round-trip: OK");

    println!("\n=== Done. Bob can now sign transactions from {} ===", addr_hex(&output.stealth_address));
}
