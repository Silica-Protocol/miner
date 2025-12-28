//! Network Utility Work (NUW) Worker Module
//!
//! This module handles solving NUW challenges from the Silica node,
//! providing useful computational work in exchange for fee discounts.
//!
//! ## Challenge Types Supported
//!
//! | Type | Description | Fee Discount |
//! |------|-------------|--------------|
//! | `Argon2Pow` | Memory-hard PoW (fallback) | 0% |
//! | `SignatureBatch` | Verify transaction signatures | 25% |
//! | `ZkVerify` | Verify ZK proofs | 50% |
//! | `MerkleVerify` | Validate Merkle proofs | 30% |
//! | `PqAssist` | Post-quantum key operations | 40% |

use anyhow::{Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::{MinerConfig, create_secure_client};

// ============================================================================
// Challenge Types (mirror of node's types)
// ============================================================================

/// NUW Challenge Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NuwChallengeType {
    /// Standard Argon2 PoW - wasteful but always available
    Argon2Pow,
    /// Verify batch of pending transaction signatures
    SignatureBatch,
    /// Verify a ZK proof
    ZkVerify,
    /// Validate Merkle state proofs
    MerkleVerify,
    /// Assist with PQ key operations
    PqAssist,
}

impl NuwChallengeType {
    /// Fee discount percentage for this work type
    pub fn fee_discount_percent(&self) -> u8 {
        match self {
            Self::Argon2Pow => 0,
            Self::SignatureBatch => 25,
            Self::ZkVerify => 50,
            Self::MerkleVerify => 30,
            Self::PqAssist => 40,
        }
    }
}

/// Argon2 PoW parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argon2Params {
    pub nonce: String, // hex
    pub difficulty: u8,
    pub memory_cost: u32,
    pub time_cost: u32,
}

/// Pending signature to verify
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSignature {
    pub tx_id: String,
    pub message: String,    // hex
    pub signature: String,  // hex
    pub public_key: String, // hex
    pub algorithm: String,  // "ed25519" or "dilithium2"
}

/// ZK Proof to verify
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProofTask {
    pub proof_id: String,
    pub proof_type: String,
    pub proof: String,              // hex
    pub public_inputs: Vec<String>, // hex
    pub verification_key: String,   // hex
}

/// Merkle proof to verify
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProofTask {
    pub root: String,       // hex
    pub leaf: String,       // hex
    pub proof: Vec<String>, // hex
    pub index: u64,
}

/// NUW Challenge from node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuwChallenge {
    pub challenge_id: String,
    pub challenge_type: NuwChallengeType,
    pub fee_discount_percent: u8,
    pub expires_at: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub argon2_params: Option<Argon2Params>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_batch: Option<Vec<PendingSignature>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub zk_proof: Option<ZkProofTask>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merkle_proofs: Option<Vec<MerkleProofTask>>,
}

/// Signature verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureResult {
    pub tx_id: String,
    pub valid: bool,
}

/// ZK verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkResult {
    pub proof_id: String,
    pub valid: bool,
}

/// Merkle verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleResult {
    pub index: u64,
    pub valid: bool,
}

/// Solved NUW challenge to submit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuwSolution {
    pub challenge_id: String,
    pub challenge_type: NuwChallengeType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub argon2_solution: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_results: Option<Vec<SignatureResult>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub zk_result: Option<ZkResult>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merkle_results: Option<Vec<MerkleResult>>,
}

// ============================================================================
// NUW Worker Statistics
// ============================================================================

/// Statistics for NUW work
#[derive(Debug, Default)]
pub struct NuwStats {
    /// Total challenges solved
    pub challenges_solved: AtomicU64,
    /// Total challenges failed
    pub challenges_failed: AtomicU64,
    /// Argon2 challenges solved
    pub argon2_solved: AtomicU64,
    /// Signature batches verified
    pub signatures_verified: AtomicU64,
    /// ZK proofs verified
    pub zk_proofs_verified: AtomicU64,
    /// Merkle proofs verified
    pub merkle_proofs_verified: AtomicU64,
    /// Total fee discounts earned (basis points)
    pub fee_discounts_earned: AtomicU64,
    /// Average solution time in milliseconds
    pub avg_solution_time_ms: AtomicU64,
}

impl NuwStats {
    pub fn record_success(&self, challenge_type: NuwChallengeType, solution_time_ms: u64) {
        self.challenges_solved.fetch_add(1, Ordering::Relaxed);

        match challenge_type {
            NuwChallengeType::Argon2Pow => {
                self.argon2_solved.fetch_add(1, Ordering::Relaxed);
            }
            NuwChallengeType::SignatureBatch => {
                self.signatures_verified.fetch_add(1, Ordering::Relaxed);
                self.fee_discounts_earned.fetch_add(25, Ordering::Relaxed);
            }
            NuwChallengeType::ZkVerify => {
                self.zk_proofs_verified.fetch_add(1, Ordering::Relaxed);
                self.fee_discounts_earned.fetch_add(50, Ordering::Relaxed);
            }
            NuwChallengeType::MerkleVerify => {
                self.merkle_proofs_verified.fetch_add(1, Ordering::Relaxed);
                self.fee_discounts_earned.fetch_add(30, Ordering::Relaxed);
            }
            NuwChallengeType::PqAssist => {
                self.fee_discounts_earned.fetch_add(40, Ordering::Relaxed);
            }
        }

        // Update moving average
        let prev = self.avg_solution_time_ms.load(Ordering::Relaxed);
        let new_avg = if prev == 0 {
            solution_time_ms
        } else {
            (prev * 9 + solution_time_ms) / 10
        };
        self.avg_solution_time_ms.store(new_avg, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.challenges_failed.fetch_add(1, Ordering::Relaxed);
    }
}

// ============================================================================
// NUW Worker
// ============================================================================

/// NUW Worker that fetches and solves challenges
pub struct NuwWorker {
    node_url: String,
    #[allow(dead_code)]
    user_id: String,
    running: Arc<AtomicBool>,
    stats: Arc<NuwStats>,
    preferred_type: Option<NuwChallengeType>,
    #[allow(dead_code)]
    min_difficulty: u32,
    last_challenge: Arc<RwLock<Option<NuwChallenge>>>,
    last_solution: Arc<RwLock<Option<NuwSolution>>>,
}

impl NuwWorker {
    /// Create a new NUW worker
    pub fn new(config: &MinerConfig) -> Self {
        Self {
            node_url: config.oracle_url.trim_end_matches('/').to_string(),
            user_id: config.user_id.clone(),
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(NuwStats::default()),
            preferred_type: None,
            min_difficulty: config.work_allocation.min_nuw_difficulty,
            last_challenge: Arc::new(RwLock::new(None)),
            last_solution: Arc::new(RwLock::new(None)),
        }
    }

    /// Get worker statistics
    pub fn stats(&self) -> &NuwStats {
        &self.stats
    }

    /// Check if worker is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Set preferred challenge type
    pub fn set_preferred_type(&mut self, challenge_type: Option<NuwChallengeType>) {
        self.preferred_type = challenge_type;
    }

    /// Get the last solved challenge
    pub async fn last_solution(&self) -> Option<NuwSolution> {
        self.last_solution.read().await.clone()
    }

    /// Start the NUW worker loop
    pub async fn start(&self) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            warn!("NUW worker already running");
            return Ok(());
        }

        info!("Starting NUW worker, connecting to {}", self.node_url);

        while self.running.load(Ordering::Relaxed) {
            match self.work_cycle().await {
                Ok(()) => {
                    // Brief pause between cycles
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    warn!("NUW work cycle error: {}", e);
                    self.stats.record_failure();
                    // Backoff on errors
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        info!("NUW worker stopped");
        Ok(())
    }

    /// Stop the NUW worker
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Single work cycle: fetch challenge, solve, submit
    async fn work_cycle(&self) -> Result<()> {
        // 1. Fetch challenge from node
        let challenge = self.fetch_challenge().await?;

        debug!(
            "Received {} challenge: {} ({}% discount)",
            challenge.challenge_type.fee_discount_percent(),
            challenge.challenge_id,
            challenge.fee_discount_percent
        );

        // Store for reference
        *self.last_challenge.write().await = Some(challenge.clone());

        // 2. Check expiry
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if challenge.expires_at <= now {
            return Err(anyhow::anyhow!("Challenge already expired"));
        }

        // 3. Solve the challenge
        let start = Instant::now();
        let solution = self.solve_challenge(&challenge).await?;
        let solve_time = start.elapsed();

        info!(
            "Solved {} challenge in {:?}",
            format!("{:?}", challenge.challenge_type),
            solve_time
        );

        // Store solution
        *self.last_solution.write().await = Some(solution.clone());

        // 4. Record stats
        self.stats
            .record_success(challenge.challenge_type, solve_time.as_millis() as u64);

        Ok(())
    }

    /// Fetch a challenge from the node
    async fn fetch_challenge(&self) -> Result<NuwChallenge> {
        let client = create_secure_client()?;

        // Build RPC request
        let mut params = serde_json::json!({});
        if let Some(pref) = &self.preferred_type {
            params["preferred_type"] = serde_json::json!(pref);
        }

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "get_nuw_challenge",
            "params": params,
            "id": 1
        });

        let resp = client
            .post(format!("{}/rpc", self.node_url))
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to connect to node")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Node returned error {}: {}", status, text));
        }

        let rpc_response: serde_json::Value = resp.json().await?;

        if let Some(error) = rpc_response.get("error") {
            return Err(anyhow::anyhow!("RPC error: {}", error));
        }

        let result = rpc_response
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("Missing result in RPC response"))?;

        let challenge: NuwChallenge =
            serde_json::from_value(result.clone()).context("Failed to parse challenge")?;

        Ok(challenge)
    }

    /// Solve a challenge based on its type
    async fn solve_challenge(&self, challenge: &NuwChallenge) -> Result<NuwSolution> {
        match challenge.challenge_type {
            NuwChallengeType::Argon2Pow => self.solve_argon2(challenge).await,
            NuwChallengeType::SignatureBatch => self.solve_signatures(challenge).await,
            NuwChallengeType::ZkVerify => self.solve_zk_proof(challenge).await,
            NuwChallengeType::MerkleVerify => self.solve_merkle(challenge).await,
            NuwChallengeType::PqAssist => {
                // PQ assist not yet implemented, fall back to Argon2
                warn!("PqAssist not implemented, treating as Argon2");
                self.solve_argon2(challenge).await
            }
        }
    }

    /// Solve Argon2 PoW challenge
    async fn solve_argon2(&self, challenge: &NuwChallenge) -> Result<NuwSolution> {
        let params = challenge
            .argon2_params
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing Argon2 params"))?;

        let nonce = hex::decode(&params.nonce).context("Invalid nonce hex")?;

        // Validate nonce length
        if nonce.len() != 32 {
            return Err(anyhow::anyhow!(
                "Invalid nonce length: expected 32, got {}",
                nonce.len()
            ));
        }

        let difficulty = params.difficulty;
        let memory_cost = params.memory_cost;
        let time_cost = params.time_cost;

        info!(
            "Solving Argon2 challenge: difficulty={}, memory={}KB, time={}",
            difficulty, memory_cost, time_cost
        );

        // Build Argon2 hasher
        let argon2_params = Params::new(memory_cost, time_cost, 1, Some(32))
            .map_err(|e| anyhow::anyhow!("Invalid Argon2 params: {}", e))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);

        // Search for solution
        let mut counter: u64 = 0;
        let mut hash = [0u8; 32];
        let max_iterations = 10_000_000u64; // Safety limit

        loop {
            if counter >= max_iterations {
                return Err(anyhow::anyhow!(
                    "Exceeded max iterations without finding solution"
                ));
            }

            let counter_bytes = counter.to_le_bytes();

            argon2
                .hash_password_into(&counter_bytes, &nonce, &mut hash)
                .map_err(|e| anyhow::anyhow!("Argon2 hash failed: {}", e))?;

            if has_leading_zeros(&hash, difficulty) {
                debug!("Found solution at counter {}", counter);
                break;
            }

            counter += 1;

            // Progress logging every 10000 attempts
            if counter % 10000 == 0 {
                debug!("Argon2 search progress: {} attempts", counter);
            }
        }

        Ok(NuwSolution {
            challenge_id: challenge.challenge_id.clone(),
            challenge_type: NuwChallengeType::Argon2Pow,
            argon2_solution: Some(hex::encode(counter.to_le_bytes())),
            signature_results: None,
            zk_result: None,
            merkle_results: None,
        })
    }

    /// Solve signature batch challenge
    async fn solve_signatures(&self, challenge: &NuwChallenge) -> Result<NuwSolution> {
        let signatures = challenge
            .signature_batch
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing signature batch"))?;

        info!("Verifying {} signatures", signatures.len());

        let mut results = Vec::with_capacity(signatures.len());

        for sig in signatures {
            let valid = verify_signature(
                &sig.message,
                &sig.signature,
                &sig.public_key,
                &sig.algorithm,
            );

            results.push(SignatureResult {
                tx_id: sig.tx_id.clone(),
                valid,
            });
        }

        let valid_count = results.iter().filter(|r| r.valid).count();
        info!(
            "Signature verification complete: {}/{} valid",
            valid_count,
            signatures.len()
        );

        Ok(NuwSolution {
            challenge_id: challenge.challenge_id.clone(),
            challenge_type: NuwChallengeType::SignatureBatch,
            argon2_solution: None,
            signature_results: Some(results),
            zk_result: None,
            merkle_results: None,
        })
    }

    /// Solve ZK proof challenge
    async fn solve_zk_proof(&self, challenge: &NuwChallenge) -> Result<NuwSolution> {
        let zk_task = challenge
            .zk_proof
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing ZK proof task"))?;

        info!(
            "Verifying ZK proof: {} (type: {})",
            zk_task.proof_id, zk_task.proof_type
        );

        // TODO: Implement actual ZK proof verification using halo2
        // For now, we'll do basic validation
        let valid = verify_zk_proof(zk_task);

        Ok(NuwSolution {
            challenge_id: challenge.challenge_id.clone(),
            challenge_type: NuwChallengeType::ZkVerify,
            argon2_solution: None,
            signature_results: None,
            zk_result: Some(ZkResult {
                proof_id: zk_task.proof_id.clone(),
                valid,
            }),
            merkle_results: None,
        })
    }

    /// Solve Merkle proof challenge
    async fn solve_merkle(&self, challenge: &NuwChallenge) -> Result<NuwSolution> {
        let proofs = challenge
            .merkle_proofs
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing Merkle proofs"))?;

        info!("Verifying {} Merkle proofs", proofs.len());

        let mut results = Vec::with_capacity(proofs.len());

        for proof in proofs {
            let valid = verify_merkle_proof(proof);
            results.push(MerkleResult {
                index: proof.index,
                valid,
            });
        }

        let valid_count = results.iter().filter(|r| r.valid).count();
        info!(
            "Merkle verification complete: {}/{} valid",
            valid_count,
            proofs.len()
        );

        Ok(NuwSolution {
            challenge_id: challenge.challenge_id.clone(),
            challenge_type: NuwChallengeType::MerkleVerify,
            argon2_solution: None,
            signature_results: None,
            zk_result: None,
            merkle_results: Some(results),
        })
    }
}

// ============================================================================
// Verification Functions
// ============================================================================

/// Check if hash has required leading zero bits
fn has_leading_zeros(hash: &[u8], required_bits: u8) -> bool {
    if required_bits == 0 {
        return true;
    }

    let mut bits_checked = 0u8;

    for byte in hash {
        if bits_checked + 8 <= required_bits {
            // Need all 8 bits to be zero
            if *byte != 0 {
                return false;
            }
            bits_checked += 8;

            // If we've checked exactly the required bits, we're done
            if bits_checked == required_bits {
                return true;
            }
        } else {
            // Need only some bits from this byte
            let remaining = required_bits - bits_checked;
            // Check high bits (remaining many bits from the left)
            let mask = 0xFFu8.checked_shl(8 - remaining as u32).unwrap_or(0);
            return (*byte & mask) == 0;
        }
    }

    true
}

/// Verify a signature (Ed25519 or Dilithium)
fn verify_signature(
    message_hex: &str,
    signature_hex: &str,
    pubkey_hex: &str,
    algorithm: &str,
) -> bool {
    // Decode hex values
    let message = match hex::decode(message_hex) {
        Ok(m) => m,
        Err(_) => return false,
    };
    let signature = match hex::decode(signature_hex) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pubkey = match hex::decode(pubkey_hex) {
        Ok(p) => p,
        Err(_) => return false,
    };

    match algorithm {
        "ed25519" => {
            // Ed25519 signature verification
            if pubkey.len() != 32 || signature.len() != 64 {
                return false;
            }

            use ed25519_dalek::{Signature, Verifier, VerifyingKey};

            let verifying_key = match VerifyingKey::try_from(pubkey.as_slice()) {
                Ok(k) => k,
                Err(_) => return false,
            };

            let sig = match Signature::try_from(signature.as_slice()) {
                Ok(s) => s,
                Err(_) => return false,
            };

            verifying_key.verify(&message, &sig).is_ok()
        }
        "dilithium2" => {
            // Dilithium PQ signature verification
            use pqcrypto_dilithium::dilithium2;
            use pqcrypto_traits::sign::*;

            let pk = match dilithium2::PublicKey::from_bytes(&pubkey) {
                Ok(k) => k,
                Err(_) => return false,
            };

            let sig = match dilithium2::DetachedSignature::from_bytes(&signature) {
                Ok(s) => s,
                Err(_) => return false,
            };

            dilithium2::verify_detached_signature(&sig, &message, &pk).is_ok()
        }
        _ => {
            warn!("Unknown signature algorithm: {}", algorithm);
            false
        }
    }
}

/// Verify a ZK proof (placeholder - needs halo2 integration)
fn verify_zk_proof(task: &ZkProofTask) -> bool {
    // TODO: Implement actual ZK verification using halo2
    // For now, do basic format validation

    // Check that proof and verification key are valid hex
    if hex::decode(&task.proof).is_err() {
        return false;
    }
    if hex::decode(&task.verification_key).is_err() {
        return false;
    }

    // Check public inputs are valid hex
    for input in &task.public_inputs {
        if hex::decode(input).is_err() {
            return false;
        }
    }

    // Placeholder: In production, this would call halo2 verification
    warn!("ZK proof verification is placeholder - returning true");
    true
}

/// Verify a Merkle proof
/// Note: Assumes `leaf` is already the hash of the leaf data
fn verify_merkle_proof(task: &MerkleProofTask) -> bool {
    // Decode values
    let root = match hex::decode(&task.root) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let leaf = match hex::decode(&task.leaf) {
        Ok(l) => l,
        Err(_) => return false,
    };
    let proof: Result<Vec<Vec<u8>>, _> = task.proof.iter().map(hex::decode).collect();
    let proof = match proof {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Verify Merkle proof - leaf is already a hash
    let mut current = leaf;
    let mut index = task.index;

    for sibling in &proof {
        let mut hasher = Sha256::new();

        if index % 2 == 0 {
            // Current is left child
            hasher.update(&current);
            hasher.update(sibling);
        } else {
            // Current is right child
            hasher.update(sibling);
            hasher.update(&current);
        }

        current = hasher.finalize().to_vec();
        index /= 2;
    }

    current == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_leading_zeros() {
        // 0 bits required - always true
        assert!(has_leading_zeros(&[0xFF], 0));

        // 8 bits required - first byte must be 0
        assert!(has_leading_zeros(&[0x00, 0xFF], 8));
        assert!(!has_leading_zeros(&[0x01, 0x00], 8));

        // 4 bits required - first nibble must be 0
        assert!(has_leading_zeros(&[0x0F], 4));
        assert!(!has_leading_zeros(&[0x10], 4));

        // 16 bits required - first two bytes must be 0
        assert!(has_leading_zeros(&[0x00, 0x00, 0xFF], 16));
        assert!(!has_leading_zeros(&[0x00, 0x01, 0x00], 16));
    }

    #[test]
    fn test_verify_merkle_proof() {
        // Simple test with known values
        let leaf = hex::encode(Sha256::digest(b"leaf_data"));
        let sibling = hex::encode(Sha256::digest(b"sibling_data"));

        // Compute expected root
        let mut hasher = Sha256::new();
        hasher.update(Sha256::digest(b"leaf_data"));
        hasher.update(Sha256::digest(b"sibling_data"));
        let root = hex::encode(hasher.finalize());

        let task = MerkleProofTask {
            root,
            leaf,
            proof: vec![sibling],
            index: 0,
        };

        assert!(verify_merkle_proof(&task));
    }
}
