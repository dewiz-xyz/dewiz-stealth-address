use k256::{
    elliptic_curve::{
        ops::Reduce,
        sec1::{FromEncodedPoint, ToEncodedPoint},
    },
    AffinePoint, EncodedPoint, NonZeroScalar, ProjectivePoint, Scalar, U256,
};
use rand_core::OsRng;
use sha3::{Digest, Keccak256};

/// Stealth meta-address: spending keypair (k, K) + viewing keypair (v, V).
pub struct StealthMetaAddress {
    pub spending_key: NonZeroScalar,
    pub viewing_key: NonZeroScalar,
    pub spending_pubkey: ProjectivePoint,
    pub viewing_pubkey: ProjectivePoint,
}

impl StealthMetaAddress {
    /// Derive a stealth meta-address from an existing EOA private key.
    ///
    /// The EOA key becomes the **spending key** (k). The **viewing key** (v)
    /// is derived deterministically:
    ///   `v = keccak256("stealth-viewing-key" || k_bytes) mod n`
    ///
    /// This lets wallet holders bootstrap ERC-5564 from their existing key
    /// without managing a second independent secret.
    pub fn from_secp256k1_nonzeroscalar(eoa_private_key: &NonZeroScalar) -> Self {
        let spending_key = *eoa_private_key;
        let spending_pubkey = ProjectivePoint::GENERATOR * **eoa_private_key;

        // Deterministic viewing key: keccak256(domain_tag || spending_key_bytes)
        let mut hasher = Keccak256::new();
        hasher.update(b"stealth-viewing-key");
        hasher.update(eoa_private_key.to_bytes());
        let hash = hasher.finalize();
        let viewing_scalar = <Scalar as Reduce<U256>>::reduce_bytes(&hash);
        // reduce_bytes can theoretically return zero for a 256-bit hash, but the
        // probability is ~1/n ≈ 2^{-256}. Treat it as unreachable.
        let viewing_key = Option::<NonZeroScalar>::from(NonZeroScalar::new(viewing_scalar))
            .expect("viewing key derived from keccak256 must be non-zero");
        let viewing_pubkey = ProjectivePoint::GENERATOR * *viewing_key;

        Self {
            spending_key,
            viewing_key,
            spending_pubkey,
            viewing_pubkey,
        }
    }

    /// Parse a hex-encoded private key string and derive a `StealthMetaAddress`.
    ///
    /// Accepts 32-byte hex (64 chars), with or without a `0x` prefix.
    /// Returns `Err` if the hex is invalid or decodes to the zero scalar.
    pub fn from_private_key_string(hex_str: &str) -> Result<Self, String> {
        let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(stripped).map_err(|e| format!("invalid hex: {e}"))?;
        if bytes.len() != 32 {
            return Err(format!("expected 32 bytes, got {}", bytes.len()));
        }
        let scalar = NonZeroScalar::try_from(bytes.as_slice())
            .map_err(|_| "decoded scalar is zero or invalid".to_string())?;
        Ok(Self::from_secp256k1_nonzeroscalar(&scalar))
    }
}

/// Stealth output after generating a stealth address.
pub struct StealthOutput {
    pub ephemeral_pubkey: ProjectivePoint,
    pub stealth_pubkey: ProjectivePoint,
    pub stealth_address: [u8; 20],
    pub view_tag: u8,
}

/// Recovered stealth private key after scanning an announcement.
pub struct RecoveredKey {
    pub stealth_private_key: Scalar,
    pub stealth_pubkey: ProjectivePoint,
    pub stealth_address: [u8; 20],
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Hash an ECDH shared secret point using SEC1 uncompressed encoding.
/// Returns (s_scalar, view_tag) where:
///   s = keccak256(0x04 || Sₓ || Sᵧ)   -- 65-byte SEC1 uncompressed input
///   view_tag = s[0]
///   s_scalar = s mod n
fn hash_shared_secret(shared_secret: &ProjectivePoint) -> (Scalar, u8) {
    let encoded = shared_secret.to_affine().to_encoded_point(false);
    let hash = Keccak256::digest(encoded.as_bytes()); // hash all 65 bytes (0x04 || x || y)
    let view_tag = hash[0];
    let scalar = <Scalar as Reduce<U256>>::reduce_bytes(&hash);
    (scalar, view_tag)
}

/// Derive an Ethereum address from a public key point.
/// addr = keccak256(Pₓ || Pᵧ)[12:32]  -- raw 64 bytes, NO 0x04 prefix
fn pubkey_to_eth_address(pubkey: &ProjectivePoint) -> [u8; 20] {
    let encoded = pubkey.to_affine().to_encoded_point(false);
    let bytes = encoded.as_bytes();
    let hash = Keccak256::digest(&bytes[1..]); // skip 0x04 tag, hash Pₓ || Pᵧ
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..32]);
    addr
}

// ---------------------------------------------------------------------------
// Protocol functions
// ---------------------------------------------------------------------------

/// Step 1: Sender generates his stealth meta-address.
///
/// k ← random, K = k·G  (spending)
/// v ← random, V = v·G  (viewing)
pub fn generate_meta_address() -> StealthMetaAddress {
    let spending_key = NonZeroScalar::random(&mut OsRng);
    let viewing_key = NonZeroScalar::random(&mut OsRng);
    let spending_pubkey = ProjectivePoint::GENERATOR * *spending_key;
    let viewing_pubkey = ProjectivePoint::GENERATOR * *viewing_key;
    StealthMetaAddress {
        spending_key,
        viewing_key,
        spending_pubkey,
        viewing_pubkey,
    }
}

/// Parse a hex-encoded compressed public key (33 bytes, "02..." or "03...") into a ProjectivePoint.
pub fn parse_pubkey_hex(hex_str: &str) -> Result<ProjectivePoint, String> {
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid hex: {e}"))?;
    let encoded =
        EncodedPoint::from_bytes(&bytes).map_err(|e| format!("invalid SEC1 encoding: {e}"))?;
    let affine: Option<AffinePoint> = AffinePoint::from_encoded_point(&encoded).into();
    affine
        .map(|pt| pt.into())
        .ok_or_else(|| "point not on curve".into())
}

/// Parse a stealth meta-address string ("st:eth:0x{K}{V}") into (spending_pubkey, viewing_pubkey).
pub fn parse_meta_address(meta_addr: &str) -> Result<(ProjectivePoint, ProjectivePoint), String> {
    let hex_payload = meta_addr
        .strip_prefix("st:eth:0x")
        .ok_or("missing st:eth:0x prefix")?;
    // Each compressed key is 33 bytes = 66 hex chars
    if hex_payload.len() != 132 {
        return Err(format!("expected 132 hex chars, got {}", hex_payload.len()));
    }
    let spending_pubkey = parse_pubkey_hex(&hex_payload[..66])?;
    let viewing_pubkey = parse_pubkey_hex(&hex_payload[66..])?;
    Ok((spending_pubkey, viewing_pubkey))
}

/// Format a meta-address as st:eth:0x{compress(K)}{compress(V)}.
pub fn format_meta_address(meta: &StealthMetaAddress) -> String {
    let k_compressed = meta.spending_pubkey.to_affine().to_encoded_point(true);
    let v_compressed = meta.viewing_pubkey.to_affine().to_encoded_point(true);
    format!(
        "st:eth:0x{}{}",
        hex::encode(k_compressed.as_bytes()),
        hex::encode(v_compressed.as_bytes()),
    )
}

/// Step 2: To generate a stealth address for Bob using his public keys.
///
/// r ← random, R = r·G
/// S = r·V                             (ECDH)
/// s = keccak256(0x04 || Sₓ || Sᵧ)    (SEC1 uncompressed)
/// P = K + s·G                         (stealth public key)
/// addr = keccak256(Pₓ || Pᵧ)[12:32]  (Ethereum address)
/// tag = s[0]                          (view tag)
pub fn generate_stealth_address(
    spending_pubkey: &ProjectivePoint,
    viewing_pubkey: &ProjectivePoint,
) -> StealthOutput {
    // 1. Ephemeral keypair
    let r = NonZeroScalar::random(&mut OsRng);
    let ephemeral_pubkey = ProjectivePoint::GENERATOR * *r;

    // 2. ECDH shared secret: S = r · V
    let shared_secret = *viewing_pubkey * *r;

    // 3-4. Hash to scalar + view tag
    let (s_scalar, view_tag) = hash_shared_secret(&shared_secret);

    // 5. Stealth public key: P = K + s·G
    let stealth_pubkey = *spending_pubkey + ProjectivePoint::GENERATOR * s_scalar;

    // 6. Ethereum address
    let stealth_address = pubkey_to_eth_address(&stealth_pubkey);

    StealthOutput {
        ephemeral_pubkey,
        stealth_pubkey,
        stealth_address,
        view_tag,
    }
}

/// Step 3: Bob scans an announcement and recovers the stealth private key.
///
/// S' = v·R
/// s' = keccak256(0x04 || S'ₓ || S'ᵧ)
/// if s'[0] ≠ tag → skip (99.6% filtered)
/// P' = K + s'·G
/// addr' = keccak256(P'ₓ || P'ᵧ)[12:32]
/// if addr' ≠ addr → skip
/// p = k + s' mod n
pub fn scan_and_recover(
    meta: &StealthMetaAddress,
    ephemeral_pubkey: &ProjectivePoint,
    view_tag: u8,
    stealth_address: &[u8; 20],
) -> Option<RecoveredKey> {
    // 1. Recompute shared secret: S' = v · R
    let shared_secret = *ephemeral_pubkey * *meta.viewing_key;

    // 2. Hash to scalar + view tag
    let (s_scalar, computed_tag) = hash_shared_secret(&shared_secret);

    // 3. View tag filter
    if computed_tag != view_tag {
        return None;
    }

    // 4. Recompute stealth public key: P' = K + s'·G
    let stealth_pubkey = meta.spending_pubkey + ProjectivePoint::GENERATOR * s_scalar;

    // 5. Recompute address
    let computed_address = pubkey_to_eth_address(&stealth_pubkey);

    // 6. Full address match
    if computed_address != *stealth_address {
        return None;
    }

    // 7. Derive stealth private key: p = k + s' mod n
    let spending_scalar: Scalar = *meta.spending_key;
    let stealth_private_key = spending_scalar + s_scalar;

    Some(RecoveredKey {
        stealth_private_key,
        stealth_pubkey,
        stealth_address: computed_address,
    })
}

/// Step 4: Verify that a stealth private key corresponds to the stealth public key.
/// p·G == P
pub fn verify(stealth_private_key: &Scalar, stealth_pubkey: &ProjectivePoint) -> bool {
    let derived = ProjectivePoint::GENERATOR * stealth_private_key;
    derived == *stealth_pubkey
}

pub fn point_to_hex(p: &ProjectivePoint) -> String {
    hex::encode(p.to_affine().to_encoded_point(true).as_bytes())
}

pub fn scalar_to_hex(s: &Scalar) -> String {
    hex::encode(s.to_bytes())
}

pub fn addr_to_hex(a: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(a))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_stealth_address_roundtrip() {
        let meta = generate_meta_address();
        let output = generate_stealth_address(&meta.spending_pubkey, &meta.viewing_pubkey);
        let recovered = scan_and_recover(
            &meta,
            &output.ephemeral_pubkey,
            output.view_tag,
            &output.stealth_address,
        );
        assert!(recovered.is_some(), "Bob should detect the payment");
        let recovered = recovered.unwrap();
        assert_eq!(recovered.stealth_address, output.stealth_address);
        assert!(verify(
            &recovered.stealth_private_key,
            &output.stealth_pubkey
        ));
    }

    #[test]
    fn wrong_view_tag_rejects() {
        let meta = generate_meta_address();
        let output = generate_stealth_address(&meta.spending_pubkey, &meta.viewing_pubkey);
        let wrong_tag = output.view_tag.wrapping_add(1);
        let result = scan_and_recover(
            &meta,
            &output.ephemeral_pubkey,
            wrong_tag,
            &output.stealth_address,
        );
        assert!(result.is_none(), "Wrong view tag should be rejected");
    }

    #[test]
    fn wrong_address_rejects() {
        let meta = generate_meta_address();
        let output = generate_stealth_address(&meta.spending_pubkey, &meta.viewing_pubkey);
        let wrong_address = [0xFFu8; 20];
        let result = scan_and_recover(
            &meta,
            &output.ephemeral_pubkey,
            output.view_tag,
            &wrong_address,
        );
        assert!(result.is_none(), "Wrong address should be rejected");
    }

    #[test]
    fn meta_address_roundtrip_parse() {
        let meta = generate_meta_address();
        let formatted = format_meta_address(&meta);
        let (k, v) = parse_meta_address(&formatted).unwrap();
        // Generate stealth address using parsed keys — should work identically
        let output = generate_stealth_address(&k, &v);
        let recovered = scan_and_recover(
            &meta,
            &output.ephemeral_pubkey,
            output.view_tag,
            &output.stealth_address,
        );
        assert!(
            recovered.is_some(),
            "Parsed meta-address should produce valid stealth addresses"
        );
        assert!(verify(
            &recovered.unwrap().stealth_private_key,
            &output.stealth_pubkey
        ));
    }

    #[test]
    fn from_secp256k1_nonzeroscalar_deterministic_and_roundtrip() {
        let eoa_key = NonZeroScalar::random(&mut OsRng);
        let meta1 = StealthMetaAddress::from_secp256k1_nonzeroscalar(&eoa_key);
        let meta2 = StealthMetaAddress::from_secp256k1_nonzeroscalar(&eoa_key);

        // Deterministic: same input key → identical meta-addresses
        assert_eq!(
            point_to_hex(&meta1.spending_pubkey),
            point_to_hex(&meta2.spending_pubkey),
        );
        assert_eq!(
            point_to_hex(&meta1.viewing_pubkey),
            point_to_hex(&meta2.viewing_pubkey),
        );

        // Spending pubkey matches the original EOA public key
        let expected_pubkey = ProjectivePoint::GENERATOR * *eoa_key;
        assert_eq!(meta1.spending_pubkey, expected_pubkey);

        // Full stealth roundtrip works with derived keys
        let output = generate_stealth_address(&meta1.spending_pubkey, &meta1.viewing_pubkey);
        let recovered = scan_and_recover(
            &meta1,
            &output.ephemeral_pubkey,
            output.view_tag,
            &output.stealth_address,
        );
        assert!(recovered.is_some(), "from_secp256k1_nonzeroscalar meta should recover stealth payments");
        let recovered = recovered.unwrap();
        assert_eq!(recovered.stealth_address, output.stealth_address);
        assert!(verify(&recovered.stealth_private_key, &output.stealth_pubkey));
    }

    #[test]
    fn from_private_key_string_roundtrip() {
        // Generate a random key, export to hex, rebuild via from_private_key_string
        let key = NonZeroScalar::random(&mut OsRng);
        let hex_str = hex::encode(key.to_bytes());

        let meta_from_key = StealthMetaAddress::from_secp256k1_nonzeroscalar(&key);
        let meta_from_hex = StealthMetaAddress::from_private_key_string(&hex_str).unwrap();

        assert_eq!(
            point_to_hex(&meta_from_key.spending_pubkey),
            point_to_hex(&meta_from_hex.spending_pubkey),
        );
        assert_eq!(
            point_to_hex(&meta_from_key.viewing_pubkey),
            point_to_hex(&meta_from_hex.viewing_pubkey),
        );

        // Also works with 0x prefix
        let prefixed = format!("0x{}", hex_str);
        let meta_prefixed = StealthMetaAddress::from_private_key_string(&prefixed).unwrap();
        assert_eq!(
            point_to_hex(&meta_from_key.spending_pubkey),
            point_to_hex(&meta_prefixed.spending_pubkey),
        );
    }

    #[test]
    fn from_private_key_string_rejects_invalid_input() {
        assert!(StealthMetaAddress::from_private_key_string("not_hex").is_err());
        assert!(StealthMetaAddress::from_private_key_string("abcd").is_err()); // too short
        assert!(StealthMetaAddress::from_private_key_string(
            "0000000000000000000000000000000000000000000000000000000000000000"
        ).is_err()); // zero scalar
    }

    #[test]
    fn different_recipients_get_different_addresses() {
        let bob = generate_meta_address();
        let carol = generate_meta_address();
        let bob_output = generate_stealth_address(&bob.spending_pubkey, &bob.viewing_pubkey);
        let carol_output = generate_stealth_address(&carol.spending_pubkey, &carol.viewing_pubkey);
        assert_ne!(bob_output.stealth_address, carol_output.stealth_address);
    }

    // -----------------------------------------------------------------------
    // Recipient isolation: wrong recipient cannot recover payment
    // -----------------------------------------------------------------------

    #[test]
    fn wrong_recipient_cannot_recover() {
        let bob = generate_meta_address();
        let carol = generate_meta_address();

        // Alice sends to Bob
        let output = generate_stealth_address(&bob.spending_pubkey, &bob.viewing_pubkey);

        // Carol tries to scan the same announcement — must fail
        let result = scan_and_recover(
            &carol,
            &output.ephemeral_pubkey,
            output.view_tag,
            &output.stealth_address,
        );
        assert!(result.is_none(), "Carol must not recover Bob's stealth payment");
    }

    // -----------------------------------------------------------------------
    // Ephemeral key uniqueness: same recipient gets distinct addresses each send
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_sends_to_same_recipient_produce_distinct_addresses() {
        let bob = generate_meta_address();
        let out1 = generate_stealth_address(&bob.spending_pubkey, &bob.viewing_pubkey);
        let out2 = generate_stealth_address(&bob.spending_pubkey, &bob.viewing_pubkey);
        let out3 = generate_stealth_address(&bob.spending_pubkey, &bob.viewing_pubkey);

        // All three stealth addresses must differ (unique ephemeral keys)
        assert_ne!(out1.stealth_address, out2.stealth_address);
        assert_ne!(out1.stealth_address, out3.stealth_address);
        assert_ne!(out2.stealth_address, out3.stealth_address);

        // All ephemeral public keys must differ
        assert_ne!(point_to_hex(&out1.ephemeral_pubkey), point_to_hex(&out2.ephemeral_pubkey));
        assert_ne!(point_to_hex(&out1.ephemeral_pubkey), point_to_hex(&out3.ephemeral_pubkey));

        // Bob can still recover each one
        for out in [&out1, &out2, &out3] {
            let recovered = scan_and_recover(
                &bob,
                &out.ephemeral_pubkey,
                out.view_tag,
                &out.stealth_address,
            );
            assert!(recovered.is_some(), "Bob should recover every stealth payment");
            assert!(verify(&recovered.unwrap().stealth_private_key, &out.stealth_pubkey));
        }
    }

    // -----------------------------------------------------------------------
    // Recovered private key derives matching Ethereum address
    // -----------------------------------------------------------------------

    #[test]
    fn recovered_key_derives_correct_eth_address() {
        let meta = generate_meta_address();
        let output = generate_stealth_address(&meta.spending_pubkey, &meta.viewing_pubkey);
        let recovered = scan_and_recover(
            &meta,
            &output.ephemeral_pubkey,
            output.view_tag,
            &output.stealth_address,
        )
        .expect("recovery should succeed");

        // Derive pubkey from recovered private key → compute address
        let derived_pubkey = ProjectivePoint::GENERATOR * recovered.stealth_private_key;
        let derived_addr = pubkey_to_eth_address(&derived_pubkey);
        assert_eq!(derived_addr, output.stealth_address, "address from recovered key must match");
    }

    // -----------------------------------------------------------------------
    // parse_pubkey_hex edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn parse_pubkey_hex_valid_compressed() {
        let meta = generate_meta_address();
        let hex_str = point_to_hex(&meta.spending_pubkey);
        let parsed = parse_pubkey_hex(&hex_str).expect("valid compressed key should parse");
        assert_eq!(
            point_to_hex(&parsed),
            hex_str,
            "round-trip through parse_pubkey_hex must be identity"
        );
    }

    #[test]
    fn parse_pubkey_hex_rejects_garbage() {
        assert!(parse_pubkey_hex("zzzz").is_err(), "non-hex input should fail");
        assert!(parse_pubkey_hex("02").is_err(), "truncated key should fail");
        assert!(parse_pubkey_hex("").is_err(), "empty string should fail");
        // 33 bytes of 0xFF — not a valid curve point
        let invalid_point = format!("02{}", "ff".repeat(32));
        assert!(parse_pubkey_hex(&invalid_point).is_err(), "off-curve point should fail");
    }

    // -----------------------------------------------------------------------
    // parse_meta_address error handling
    // -----------------------------------------------------------------------

    #[test]
    fn parse_meta_address_rejects_wrong_prefix() {
        assert!(parse_meta_address("wrong:prefix:0xabc").is_err());
        assert!(parse_meta_address("st:btc:0xabc").is_err());
        assert!(parse_meta_address("").is_err());
    }

    #[test]
    fn parse_meta_address_rejects_wrong_length() {
        // Correct prefix but payload too short
        assert!(parse_meta_address("st:eth:0xabcdef").is_err());
        // Correct prefix but payload too long (134 hex chars)
        let too_long = format!("st:eth:0x{}", "aa".repeat(67));
        assert!(parse_meta_address(&too_long).is_err());
    }

    // -----------------------------------------------------------------------
    // format + parse meta-address consistency with from_secp256k1_nonzeroscalar
    // -----------------------------------------------------------------------

    #[test]
    fn format_parse_meta_address_with_derived_keys() {
        let key = NonZeroScalar::random(&mut OsRng);
        let meta = StealthMetaAddress::from_secp256k1_nonzeroscalar(&key);
        let formatted = format_meta_address(&meta);

        // Must have the canonical prefix
        assert!(formatted.starts_with("st:eth:0x"));

        let (k_parsed, v_parsed) = parse_meta_address(&formatted).unwrap();
        assert_eq!(point_to_hex(&k_parsed), point_to_hex(&meta.spending_pubkey));
        assert_eq!(point_to_hex(&v_parsed), point_to_hex(&meta.viewing_pubkey));

        // Stealth roundtrip through parsed keys
        let output = generate_stealth_address(&k_parsed, &v_parsed);
        let recovered = scan_and_recover(
            &meta,
            &output.ephemeral_pubkey,
            output.view_tag,
            &output.stealth_address,
        );
        assert!(recovered.is_some());
    }

    // -----------------------------------------------------------------------
    // from_secp256k1_nonzeroscalar: viewing key ≠ spending key
    // -----------------------------------------------------------------------

    #[test]
    fn from_secp256k1_nonzeroscalar_viewing_key_differs_from_spending_key() {
        let key = NonZeroScalar::random(&mut OsRng);
        let meta = StealthMetaAddress::from_secp256k1_nonzeroscalar(&key);

        assert_ne!(
            scalar_to_hex(&meta.spending_key),
            scalar_to_hex(&meta.viewing_key),
            "viewing key must differ from spending key"
        );
        assert_ne!(
            point_to_hex(&meta.spending_pubkey),
            point_to_hex(&meta.viewing_pubkey),
            "public keys must also differ"
        );
    }

    // -----------------------------------------------------------------------
    // from_private_key_string: uppercase hex accepted
    // -----------------------------------------------------------------------

    #[test]
    fn from_private_key_string_accepts_uppercase() {
        let key = NonZeroScalar::random(&mut OsRng);
        let hex_lower = hex::encode(key.to_bytes());
        let hex_upper = hex_lower.to_uppercase();

        let meta_lower = StealthMetaAddress::from_private_key_string(&hex_lower).unwrap();
        let meta_upper = StealthMetaAddress::from_private_key_string(&hex_upper).unwrap();

        assert_eq!(
            point_to_hex(&meta_lower.spending_pubkey),
            point_to_hex(&meta_upper.spending_pubkey),
        );
    }

    // -----------------------------------------------------------------------
    // Utility function output format
    // -----------------------------------------------------------------------

    #[test]
    fn addr_to_hex_has_0x_prefix_and_correct_length() {
        let meta = generate_meta_address();
        let output = generate_stealth_address(&meta.spending_pubkey, &meta.viewing_pubkey);
        let hex_addr = addr_to_hex(&output.stealth_address);

        assert!(hex_addr.starts_with("0x"), "address must have 0x prefix");
        assert_eq!(hex_addr.len(), 42, "0x + 40 hex chars = 42"); // 0x + 20 bytes * 2
    }

    #[test]
    fn point_to_hex_is_compressed_33_bytes() {
        let meta = generate_meta_address();
        let hex_str = point_to_hex(&meta.spending_pubkey);

        assert_eq!(hex_str.len(), 66, "compressed point = 33 bytes = 66 hex chars");
        assert!(
            hex_str.starts_with("02") || hex_str.starts_with("03"),
            "compressed SEC1 must start with 02 or 03"
        );
    }

    #[test]
    fn scalar_to_hex_is_32_bytes() {
        let key = NonZeroScalar::random(&mut OsRng);
        let hex_str = scalar_to_hex(&key);
        assert_eq!(hex_str.len(), 64, "scalar = 32 bytes = 64 hex chars");
    }

    // -----------------------------------------------------------------------
    // View tag collision: correct tag but mismatched address still rejects
    // -----------------------------------------------------------------------

    #[test]
    fn correct_view_tag_but_wrong_address_still_rejects() {
        let bob = generate_meta_address();
        let output = generate_stealth_address(&bob.spending_pubkey, &bob.viewing_pubkey);

        // Tamper the address while keeping the correct view tag
        let mut tampered_addr = output.stealth_address;
        tampered_addr[0] ^= 0xFF;
        tampered_addr[19] ^= 0xFF;

        let result = scan_and_recover(
            &bob,
            &output.ephemeral_pubkey,
            output.view_tag,
            &tampered_addr,
        );
        assert!(result.is_none(), "tampered address must be rejected even with correct view tag");
    }

    // -----------------------------------------------------------------------
    // Stealth address is a valid 20-byte Ethereum address
    // -----------------------------------------------------------------------

    #[test]
    fn stealth_address_is_nonzero() {
        let meta = generate_meta_address();
        let output = generate_stealth_address(&meta.spending_pubkey, &meta.viewing_pubkey);
        assert_ne!(output.stealth_address, [0u8; 20], "stealth address must not be zero");
    }

    // -----------------------------------------------------------------------
    // Verify rejects wrong private key
    // -----------------------------------------------------------------------

    #[test]
    fn verify_rejects_wrong_private_key() {
        let meta = generate_meta_address();
        let output = generate_stealth_address(&meta.spending_pubkey, &meta.viewing_pubkey);
        let wrong_scalar: Scalar = *NonZeroScalar::random(&mut OsRng);
        assert!(
            !verify(&wrong_scalar, &output.stealth_pubkey),
            "verify must reject a random wrong key"
        );
    }

    // -----------------------------------------------------------------------
    // from_private_key_string rejects order-exceeding scalars and edge values
    // -----------------------------------------------------------------------

    #[test]
    fn from_private_key_string_rejects_too_short_and_too_long() {
        // 31 bytes
        let short = hex::encode([0xABu8; 31]);
        assert!(StealthMetaAddress::from_private_key_string(&short).is_err());

        // 33 bytes
        let long = hex::encode([0xABu8; 33]);
        assert!(StealthMetaAddress::from_private_key_string(&long).is_err());
    }

    #[test]
    fn from_private_key_string_accepts_known_key() {
        // Well-known 32-byte key (just below curve order)
        let known_hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let meta = StealthMetaAddress::from_private_key_string(known_hex).expect("scalar 1 is valid");

        // G point as spending pubkey (since key = 1, pubkey = 1·G = G)
        let generator = ProjectivePoint::GENERATOR;
        assert_eq!(
            point_to_hex(&meta.spending_pubkey),
            point_to_hex(&generator),
            "private key 1 should yield generator point as pubkey"
        );
    }
}
