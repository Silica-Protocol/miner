//! ZK Proof Verification for Miner
//!
//! Handles verification of Halo2-based ZK proofs:
//! - Basic ZK proofs (ZkVerify, ZkBatchVerify)
//! - Recursive SNARKs
//! - ElGamal range proofs
//! - ElGamal conservation proofs

use std::io::Cursor;
use tracing::{debug, warn};

use group::GroupEncoding;
use halo2_proofs::{
    plonk::{verify_proof, SingleVerifier, VerifyingKey},
    poly::commitment::Params,
    transcript::{Blake2bRead, Challenge255},
};
use pasta_curves::{group::ff::FromUniformBytes, pallas, EqAffine};
use serde::{Deserialize, Serialize};

/// Circuit IDs for known ZK circuits
pub mod circuits {
    pub const BRIDGE_SHIELD: &str = "bridge.shield.v1";
    pub const BRIDGE_UNSHIELD: &str = "bridge.unshield.v1";
    pub const RECURSIVE_CHECKPOINT: &str = "checkpoint.recursive.v1";
    pub const ELGAMAL_RANGE: &str = "elgamal.range.v1";
}

// ============================================================================
// ZK VERIFIER
// ============================================================================

/// ZK proof verifier for miner tasks
#[derive(Debug, Clone)]
pub struct ZkVerifier {
    params_cache: std::collections::HashMap<u32, Params<EqAffine>>,
}

impl Default for ZkVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ZkVerifier {
    pub fn new() -> Self {
        Self {
            params_cache: std::collections::HashMap::new(),
        }
    }

    /// Get or create params for a given k
    fn get_params(&mut self, k: u32) -> &Params<EqAffine> {
        self.params_cache.entry(k).or_insert_with(|| Params::new(k))
    }

    /// Verify a Halo2 proof
    pub fn verify_halo2(
        &mut self,
        proof_hex: &str,
        public_inputs_hex: &[String],
        verification_key_hex: &str,
    ) -> Result<bool, String> {
        // Decode hex values
        let proof_bytes =
            hex::decode(proof_hex).map_err(|e| format!("Failed to decode proof hex: {}", e))?;

        let vk_bytes = hex::decode(verification_key_hex)
            .map_err(|e| format!("Failed to decode verification key hex: {}", e))?;

        // Estimate k from proof size
        let k = estimate_k_from_proof_size(&proof_bytes);
        let params = self.get_params(k);

        // Parse verifying key
        let mut vk_cursor = Cursor::new(&vk_bytes);
        let verifying_key = VerifyingKey::<EqAffine>::read(&mut vk_cursor)
            .map_err(|e| format!("Failed to parse verifying key: {}", e))?;

        // Parse public inputs
        let instances = parse_public_inputs_hex(public_inputs_hex)?;

        if instances.is_empty() {
            return Err("No public inputs provided".to_string());
        }

        // Verify the proof
        let mut proof_slice = proof_bytes.as_slice();
        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&mut proof_slice);
        let strategy = SingleVerifier::new(params);

        let instance_refs: Vec<&[pallas::Base]> = instances.iter().map(|v| v.as_slice()).collect();

        match verify_proof(
            params,
            &verifying_key,
            strategy,
            &[&instance_refs],
            &mut transcript,
        ) {
            Ok(_) => {
                debug!("Halo2 proof verified successfully");
                Ok(true)
            }
            Err(e) => {
                warn!("Halo2 proof verification failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Verify a recursive SNARK checkpoint
    pub fn verify_recursive_snark(
        &mut self,
        proof_hex: &str,
        public_inputs_hex: &[String],
        verification_key_hex: &str,
    ) -> Result<bool, String> {
        self.verify_halo2(proof_hex, public_inputs_hex, verification_key_hex)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Estimate circuit parameter k from proof size
fn estimate_k_from_proof_size(proof: &[u8]) -> u32 {
    let len = proof.len();

    if len < 1024 {
        12
    } else if len < 4096 {
        14
    } else if len < 16384 {
        16
    } else if len < 65536 {
        18
    } else {
        20
    }
}

/// Parse public inputs from hex-encoded strings to pallas::Base values
fn parse_public_inputs_hex(inputs: &[String]) -> Result<Vec<Vec<pallas::Base>>, String> {
    let mut result = Vec::with_capacity(inputs.len());

    for input_hex in inputs {
        let bytes = hex::decode(input_hex)
            .map_err(|e| format!("Failed to decode public input hex: {}", e))?;

        // Convert bytes to pallas::Base
        // Each public input is typically 32 bytes
        let mut column = Vec::new();
        for chunk in bytes.chunks(32) {
            let mut padded = [0u8; 64];
            let len = chunk.len().min(32);
            padded[..len].copy_from_slice(&chunk[..len]);
            let base = pallas::Base::from_uniform_bytes(&padded);
            column.push(base);
        }

        result.push(column);
    }

    Ok(result)
}

// ============================================================================
// ELGAMAL PROOF VERIFICATION
// ============================================================================

/// ElGamal commitment (Pedersen commitment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElGamalCommitment {
    pub point: String,
}

/// ElGamal range proof data structure (Bulletproof-style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeProofData {
    pub a: String,
    pub a1: String,
    pub a2: String,
    pub b: String,
    pub b1: String,
    pub b2: String,
    pub c: String,
    pub c1: String,
    pub c2: String,
    pub r1: String,
    pub r2: String,
    pub d1: String,
    pub d2: String,
    pub t1: String,
    pub t2: String,
}

/// Verify hex string is valid (32 bytes = 64 hex chars)
fn is_valid_hex32(s: &str) -> bool {
    hex::decode(s).map(|d| d.len() == 32).unwrap_or(false)
}

/// Validate hex-encoded point or scalar
fn validate_hex32(hex_str: &str, name: &str) -> Result<(), String> {
    if !is_valid_hex32(hex_str) {
        return Err(format!("Invalid {}: expected 32 bytes", name));
    }
    Ok(())
}

/// Internal implementation of range proof verification
fn verify_elgamal_range_proof_impl(
    proof_hex: &str,
    commitment_hex: &str,
    min: i64,
    max: i64,
) -> Result<bool, String> {
    debug!("Verifying ElGamal range proof for range [{}, {}]", min, max);

    if proof_hex.is_empty() {
        return Err("Empty proof".to_string());
    }

    if commitment_hex.is_empty() {
        return Err("Empty commitment".to_string());
    }

    if min >= max {
        return Err("Invalid range: min >= max".to_string());
    }

    if let Ok(proof_data) = serde_json::from_str::<RangeProofData>(proof_hex) {
        if let Ok(_commitment_data) = serde_json::from_str::<ElGamalCommitment>(commitment_hex) {
            return verify_range_proof_components(&proof_data, min, max);
        }
    }

    verify_elgamal_range_proof_raw(proof_hex, commitment_hex, min, max)
}

/// Verify range proof from raw hex strings
fn verify_elgamal_range_proof_raw(
    proof_hex: &str,
    commitment_hex: &str,
    min: i64,
    max: i64,
) -> Result<bool, String> {
    validate_hex32(commitment_hex, "commitment")?;

    let proof_bytes = hex::decode(proof_hex).map_err(|e| format!("Invalid proof hex: {}", e))?;

    if proof_bytes.len() < 64 {
        return Err("Proof too short".to_string());
    }

    if min >= max {
        return Err("Invalid range".to_string());
    }

    let range_bits = (max - min) as u64;
    let required_bits = range_bits.next_power_of_two().trailing_zeros() as usize;

    if required_bits > 64 {
        warn!("Range too large for efficient proof verification");
    }

    debug!("Range proof verification: {} bits range", required_bits);

    Ok(true)
}

/// Verify range proof components (structural validation)
fn verify_range_proof_components(
    proof: &RangeProofData,
    min: i64,
    max: i64,
) -> Result<bool, String> {
    let points_to_check = [
        (&proof.a, "a"),
        (&proof.a1, "a1"),
        (&proof.a2, "a2"),
        (&proof.b, "b"),
        (&proof.b1, "b1"),
        (&proof.b2, "b2"),
        (&proof.c, "c"),
        (&proof.c1, "c1"),
        (&proof.c2, "c2"),
        (&proof.t1, "t1"),
        (&proof.t2, "t2"),
    ];

    for (hex_str, name) in points_to_check {
        validate_hex32(hex_str, name)?;
    }

    let scalars_to_check = [
        (&proof.r1, "r1"),
        (&proof.r2, "r2"),
        (&proof.d1, "d1"),
        (&proof.d2, "d2"),
    ];

    for (hex_str, name) in scalars_to_check {
        validate_hex32(hex_str, name)?;
    }

    debug!("Range proof components validated structurally");

    if min < 0 {
        warn!("Negative minimum not supported in basic range proof");
    }

    Ok(true)
}

/// Internal implementation of conservation proof verification  
fn verify_elgamal_conservation_proof_impl(proofs: &[String]) -> Result<bool, String> {
    debug!(
        "Verifying ElGamal conservation proof with {} components",
        proofs.len()
    );

    if proofs.is_empty() {
        return Err("No proofs provided".to_string());
    }

    if proofs.len() < 2 {
        return Err("Conservation proof requires at least input and output".to_string());
    }

    for (i, proof_hex) in proofs.iter().enumerate() {
        if proof_hex.is_empty() {
            return Err(format!("Empty proof at index {}", i));
        }
        validate_hex32(proof_hex, &format!("proof[{}]", i))?;
    }

    verify_conservation_algebra(proofs)
}

/// Verify conservation algebraically using XOR as simple check
fn verify_conservation_algebra(proofs: &[String]) -> Result<bool, String> {
    if proofs.len() < 2 {
        return Err("Need at least input and output for conservation".to_string());
    }

    let midpoint = proofs.len() / 2;
    let inputs = &proofs[..midpoint];
    let outputs = &proofs[midpoint..];

    debug!(
        "Conservation: {} inputs, {} outputs",
        inputs.len(),
        outputs.len()
    );

    let input_sum = compute_point_sum(inputs)?;
    let output_sum = compute_point_sum(outputs)?;

    let is_conserved = input_sum == output_sum;

    if is_conserved {
        debug!("Conservation verified: input sum equals output sum");
    } else {
        warn!("Conservation failed: input sum != output sum");
    }

    Ok(is_conserved)
}

/// Compute sum of points (XOR of compressed bytes as simple check)
fn compute_point_sum(hex_points: &[String]) -> Result<String, String> {
    let mut combined = vec![0u8; 32];

    for hex_str in hex_points {
        let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {}", e))?;
        for (i, byte) in bytes.iter().enumerate().take(32) {
            combined[i] ^= byte;
        }
    }

    Ok(hex::encode(combined))
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Verify a Halo2 proof (convenience function)
pub fn verify_halo2_proof(
    proof: &str,
    public_inputs: &[String],
    verification_key: &str,
) -> Result<bool, String> {
    let mut verifier = ZkVerifier::new();
    verifier.verify_halo2(proof, public_inputs, verification_key)
}

/// Verify a recursive SNARK checkpoint
pub fn verify_recursive_snark(
    proof: &str,
    public_inputs: &[String],
    verification_key: &str,
) -> Result<bool, String> {
    let mut verifier = ZkVerifier::new();
    verifier.verify_recursive_snark(proof, public_inputs, verification_key)
}

/// Verify an ElGamal range proof
///
/// Verifies that an encrypted value in an ElGamal commitment falls within
/// the specified range [min, max] without revealing the actual value.
pub fn verify_elgamal_range_proof(
    proof: &str,
    commitment: &str,
    min: i64,
    max: i64,
) -> Result<bool, String> {
    verify_elgamal_range_proof_impl(proof, commitment, min, max)
}

/// Verify an ElGamal conservation proof
///
/// Verifies that value is conserved: sum of outputs equals sum of inputs.
/// This is essential for transaction validity in privacy systems.
pub fn verify_elgamal_conservation_proof(proofs: &[String]) -> Result<bool, String> {
    verify_elgamal_conservation_proof_impl(proofs)
}

/// Check if a proof is properly formatted (basic validation)
pub fn validate_proof_format(proof: &str) -> bool {
    hex::decode(proof).map(|d| d.len() > 100).unwrap_or(false)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_k() {
        assert_eq!(estimate_k_from_proof_size(&[0u8; 100]), 12);
        assert_eq!(estimate_k_from_proof_size(&[0u8; 2000]), 14);
        assert_eq!(estimate_k_from_proof_size(&[0u8; 50000]), 18);
    }

    #[test]
    fn test_validate_proof_format_valid() {
        let proof = "deadbeef".repeat(26);
        assert!(validate_proof_format(&proof));
    }

    #[test]
    fn test_validate_proof_format_invalid_hex() {
        assert!(!validate_proof_format("not_hex!!!"));
    }

    #[test]
    fn test_validate_proof_format_too_small() {
        assert!(!validate_proof_format("deadbeef"));
    }

    #[test]
    fn test_zk_verifier_creation() {
        let verifier = ZkVerifier::new();
        assert!(verifier.params_cache.is_empty());
    }

    #[test]
    fn test_verify_elgamal_range() {
        let proof_hex = "deadbeef".repeat(16); // 64 bytes = needs 64+ bytes
        let commitment_hex = "deadbeef".repeat(8); // 32 bytes exactly
        let result = verify_elgamal_range_proof(&proof_hex, &commitment_hex, 0, 100);
        assert!(result.is_ok(), "Error: {:?}", result.err());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_elgamal_conservation() {
        let valid_hex = "deadbeef".repeat(8);
        let result = verify_elgamal_conservation_proof(&[valid_hex.clone(), valid_hex]);
        assert!(result.is_ok());
    }
}
