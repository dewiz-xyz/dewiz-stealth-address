use k256::{
    elliptic_curve::{
        ops::Reduce,
        sec1::{FromEncodedPoint, ToEncodedPoint},
    },
    AffinePoint, EncodedPoint, NonZeroScalar, ProjectivePoint, Scalar, U256,
};
use rand_core::OsRng;
use sha3::{Digest, Keccak256};

/// Bob's stealth meta-address: spending keypair (k, K) + viewing keypair (v, V).
pub struct StealthMetaAddress {
    pub spending_key: NonZeroScalar,
    pub viewing_key: NonZeroScalar,
    pub spending_pubkey: ProjectivePoint,
    pub viewing_pubkey: ProjectivePoint,
}

/// Alice's output after generating a stealth address for Bob.
pub struct StealthOutput {
    pub ephemeral_pubkey: ProjectivePoint,
    pub stealth_pubkey: ProjectivePoint,
    pub stealth_address: [u8; 20],
    pub view_tag: u8,
}

/// Bob's recovered stealth private key after scanning an announcement.
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

/// Step 1: Bob generates his stealth meta-address.
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

/// Step 2: Alice generates a stealth address for Bob using his public keys.
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
    fn different_recipients_get_different_addresses() {
        let bob = generate_meta_address();
        let carol = generate_meta_address();
        let bob_output = generate_stealth_address(&bob.spending_pubkey, &bob.viewing_pubkey);
        let carol_output = generate_stealth_address(&carol.spending_pubkey, &carol.viewing_pubkey);
        assert_ne!(bob_output.stealth_address, carol_output.stealth_address);
    }
}
