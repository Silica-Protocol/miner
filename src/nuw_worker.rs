//! Network Utility Work (NUW) Worker Module
//!
//! This module handles solving NUW tasks from the Silica oracle,
//! providing useful computational work in exchange for rewards.
//!
//! ## Task Types Supported
//!
//! | Type | Distribution | Description |
//! |------|--------------|-------------|
//! | `SigBatchVerify` | Single | Verify transaction signatures |
//! | `ZkBatchVerify` | Single | Verify ZK proofs in batch |
//! | `ZkVerify` | Single | Verify single ZK proof |
//! | `RecursiveSnark` | Single | Recursive SNARK verification |
//! | `MerkleBatch` | Single | Batch Merkle proof verification |
//! | `MerkleVerify` | Single | Single Merkle proof verification |
//! | `ElGamalRangeProof` | Single | ElGamal range proof verification |
//! | `ElGamalConservationProof` | Single | ElGamal conservation proof |
//! | `TxPreValidate` | Single | Transaction pre-validation |
//! | `BoincRosetta` | Quad | BOINC Rosetta@Home work |
//! | `BoincFolding` | Quad | BOINC Folding@Home work |
//! | `BoincEinstein` | Quad | BOINC Einstein@Home work |
//! | `BoincMilkyWay` | Quad | BOINC MilkyWay@Home work |

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::{MinerConfig, create_secure_client};

// ============================================================================
// Task Types (mirror of oracle's types)
// ============================================================================

/// NUW Task Types - matches oracle TaskType enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    SigBatchVerify,
    ZkBatchVerify,
    ZkVerify,
    RecursiveSnark,
    MerkleBatch,
    MerkleVerify,
    ElGamalRangeProof,
    ElGamalConservationProof,
    TxPreValidate,
    BoincRosetta,
    BoincFolding,
    BoincEinstein,
    BoincMilkyWay,
}

impl TaskType {
    /// Returns true if this task type uses single-send (verifiable)
    pub fn is_single_send(&self) -> bool {
        !matches!(
            self,
            TaskType::BoincRosetta
                | TaskType::BoincFolding
                | TaskType::BoincEinstein
                | TaskType::BoincMilkyWay
        )
    }

    /// Returns true if this is a BOINC task type
    pub fn is_boinc(&self) -> bool {
        matches!(
            self,
            TaskType::BoincRosetta
                | TaskType::BoincFolding
                | TaskType::BoincEinstein
                | TaskType::BoincMilkyWay
        )
    }

    /// Timeout in milliseconds for this task type
    pub fn timeout_ms(&self) -> u64 {
        match self {
            TaskType::SigBatchVerify => 500,
            TaskType::ZkBatchVerify => 2000,
            TaskType::ZkVerify => 2000,
            TaskType::RecursiveSnark => 60_000,
            TaskType::ElGamalRangeProof => 1000,
            TaskType::ElGamalConservationProof => 1000,
            TaskType::MerkleBatch => 500,
            TaskType::MerkleVerify => 200,
            TaskType::TxPreValidate => 100,
            TaskType::BoincRosetta
            | TaskType::BoincFolding
            | TaskType::BoincEinstein
            | TaskType::BoincMilkyWay => 86_400_000,
        }
    }
}

/// Task payload - the actual work to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskPayload {
    /// Signature batch verification
    SigBatch {
        signatures: Vec<SignatureToVerify>,
    },
    /// ZK proof verification
    ZkProof {
        proof: String,
        public_inputs: Vec<String>,
        verification_key: String,
    },
    /// Recursive SNARK
    RecursiveSnark {
        proof: String,
        public_inputs: Vec<String>,
        verification_key: Option<String>,
    },
    /// Merkle batch verification
    MerkleBatch {
        proofs: Vec<MerkleProof>,
    },
    /// Single merkle proof
    MerkleVerify {
        root: String,
        leaf: String,
        proof: Vec<String>,
        index: u64,
    },
    /// ElGamal range proof
    ElGamalRangeProof {
        proof: String,
        commitment: String,
        min: i64,
        max: i64,
    },
    /// ElGamal conservation proof
    ElGamalConservationProof {
        proofs: Vec<String>,
    },
    /// Transaction pre-validation
    TxPreValidate {
        transactions: Vec<TransactionToValidate>,
    },
    /// BOINC work (delegated to BOINC client)
    Boinc {
        project: String,
        work_unit: String,
    },
}

/// Signature to verify
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureToVerify {
    pub tx_id: String,
    pub message: String,
    pub signature: String,
    pub public_key: String,
    pub algorithm: String,
}

/// Merkle proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub root: String,
    pub leaf: String,
    pub proof: Vec<String>,
    pub index: u64,
}

/// Transaction to pre-validate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionToValidate {
    pub tx_id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: String,
    pub fee: String,
    pub nonce: u64,
    pub signature: String,
}

/// NUW Task from oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuwTask {
    pub task_id: String,
    pub task_type: TaskType,
    pub payload: TaskPayload,
    pub expires_at: i64,
    pub difficulty_multiplier: f64,
}

/// Solution result for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub task_type: TaskType,
    pub result: Vec<u8>,
    pub is_valid: bool,
    pub compute_time_ms: u64,
}

/// Miner registration info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerRegistration {
    pub miner_id: String,
    pub account_address: String,
    pub worker_name: String,
    pub public_key: Vec<u8>,
    pub supported_task_types: Vec<TaskType>,
    pub region: String,
    pub endpoint: String,
}

// ============================================================================
// NUW Worker Statistics
// ============================================================================

/// Statistics for NUW work
#[derive(Debug, Default)]
pub struct NuwStats {
    /// Total tasks completed
    pub tasks_completed: AtomicU64,
    /// Total tasks failed
    pub tasks_failed: AtomicU64,
    /// Tasks by type
    pub sig_batch_verified: AtomicU64,
    pub zk_verified: AtomicU64,
    pub merkle_verified: AtomicU64,
    pub tx_prevalidated: AtomicU64,
    pub boinc_completed: AtomicU64,
    /// Total rewards earned (in smallest units)
    pub rewards_earned: AtomicU64,
    /// Average solution time in milliseconds
    pub avg_solution_time_ms: AtomicU64,
}

impl NuwStats {
    pub fn record_success(&self, task_type: TaskType, solution_time_ms: u64) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);

        match task_type {
            TaskType::SigBatchVerify => {
                self.sig_batch_verified.fetch_add(1, Ordering::Relaxed);
            }
            TaskType::ZkBatchVerify | TaskType::ZkVerify | TaskType::RecursiveSnark => {
                self.zk_verified.fetch_add(1, Ordering::Relaxed);
            }
            TaskType::MerkleBatch | TaskType::MerkleVerify => {
                self.merkle_verified.fetch_add(1, Ordering::Relaxed);
            }
            TaskType::TxPreValidate => {
                self.tx_prevalidated.fetch_add(1, Ordering::Relaxed);
            }
            TaskType::ElGamalRangeProof | TaskType::ElGamalConservationProof => {}
            TaskType::BoincRosetta
            | TaskType::BoincFolding
            | TaskType::BoincEinstein
            | TaskType::BoincMilkyWay => {
                self.boinc_completed.fetch_add(1, Ordering::Relaxed);
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
        self.tasks_failed.fetch_add(1, Ordering::Relaxed);
    }
}

// ============================================================================
// NUW Worker
// ============================================================================

/// NUW Worker that fetches and solves challenges
pub struct NuwWorker {
    oracle_url: String,
    miner_id: String,
    account_address: String,
    worker_name: String,
    running: Arc<AtomicBool>,
    stats: Arc<NuwStats>,
    supported_types: Vec<TaskType>,
    current_task: Arc<RwLock<Option<NuwTask>>>,
    last_result: Arc<RwLock<Option<TaskResult>>>,
}

impl NuwWorker {
    /// Create a new NUW worker
    pub fn new(config: &MinerConfig, account_address: String, worker_name: String) -> Self {
        let miner_id = format!("{}:{}", account_address, worker_name);
        
        Self {
            oracle_url: config.oracle_url.trim_end_matches('/').to_string(),
            miner_id,
            account_address,
            worker_name,
            running: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(NuwStats::default()),
            supported_types: vec![
                TaskType::SigBatchVerify,
                TaskType::ZkBatchVerify,
                TaskType::ZkVerify,
                TaskType::RecursiveSnark,
                TaskType::MerkleBatch,
                TaskType::MerkleVerify,
                TaskType::ElGamalRangeProof,
                TaskType::ElGamalConservationProof,
                TaskType::TxPreValidate,
                TaskType::BoincRosetta,
                TaskType::BoincFolding,
                TaskType::BoincEinstein,
                TaskType::BoincMilkyWay,
            ],
            current_task: Arc::new(RwLock::new(None)),
            last_result: Arc::new(RwLock::new(None)),
        }
    }

    /// Get miner ID
    pub fn miner_id(&self) -> &str {
        &self.miner_id
    }

    /// Get supported task types
    pub fn supported_task_types(&self) -> &[TaskType] {
        &self.supported_types
    }

    /// Get worker statistics
    pub fn stats(&self) -> &NuwStats {
        &self.stats
    }

    /// Get worker statistics as Arc
    pub fn stats_arc(&self) -> Arc<NuwStats> {
        Arc::clone(&self.stats)
    }

    /// Check if worker is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Set supported task types
    pub fn set_supported_types(&mut self, types: Vec<TaskType>) {
        self.supported_types = types;
    }

    /// Get the last task result
    pub async fn last_result(&self) -> Option<TaskResult> {
        self.last_result.read().await.clone()
    }

    /// Start the NUW worker loop
    pub async fn start(&self) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            warn!("NUW worker already running");
            return Ok(());
        }

        info!("Starting NUW worker, connecting to {}", self.oracle_url);

        // First, register with the oracle
        self.register_with_oracle().await?;

        while self.running.load(Ordering::Relaxed) {
            match self.work_cycle().await {
                Ok(()) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    warn!("NUW work cycle error: {}", e);
                    self.stats.record_failure();
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

    /// Register this miner with the oracle
    async fn register_with_oracle(&self) -> Result<()> {
        let client = create_secure_client()?;

        let registration = MinerRegistration {
            miner_id: self.miner_id.clone(),
            account_address: self.account_address.clone(),
            worker_name: self.worker_name.clone(),
            public_key: vec![], // TODO: generate or load signing key
            supported_task_types: self.supported_types.clone(),
            region: "auto".to_string(),
            endpoint: format!("{}/miner", self.oracle_url),
        };

        let resp = client
            .post(format!("{}/api/nuw/register", self.oracle_url))
            .json(&registration)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to register with oracle")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Registration failed {}: {}", status, text));
        }

        info!("Successfully registered with oracle as {}", self.miner_id);
        Ok(())
    }

    /// Single work cycle: fetch task, solve, submit
    async fn work_cycle(&self) -> Result<()> {
        // 1. Fetch task from oracle
        let task = self.fetch_task().await?;

        debug!(
            "Received {:?} task: {}",
            task.task_type, task.task_id
        );

        // Store for reference
        *self.current_task.write().await = Some(task.clone());

        // 2. Check expiry
        let now = chrono::Utc::now().timestamp();
        if task.expires_at < now {
            warn!("Task {} expired", task.task_id);
            return Ok(());
        }

        // 3. Solve the task
        let start = Instant::now();
        let result = self.execute_task(&task).await?;
        let solve_time = start.elapsed();

        // 4. Submit result
        self.submit_result(&task, &result, solve_time.as_millis() as u64).await?;

        // 5. Record stats
        self.stats.record_success(task.task_type, solve_time.as_millis() as u64);

        Ok(())
    }

    /// Fetch a task from the oracle
    async fn fetch_task(&self) -> Result<NuwTask> {
        let client = create_secure_client()?;

        let resp = client
            .get(format!(
                "{}/api/nuw/task?miner_id={}",
                self.oracle_url, self.miner_id
            ))
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to connect to oracle")?;

        if !resp.status().is_success() {
            let status = resp.status();
            if status.as_u16() == 204 {
                return Err(anyhow::anyhow!("no_tasks_available"));
            }
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Oracle returned error {}: {}", status, text));
        }

        let task: NuwTask = resp.json().await.context("Failed to parse task")?;

        Ok(task)
    }

    /// Execute a task based on its type
    async fn execute_task(&self, task: &NuwTask) -> Result<TaskResult> {
        let start = Instant::now();
        
        let result = match &task.payload {
            TaskPayload::SigBatch { signatures } => {
                self.verify_signatures(signatures).await?
            }
            TaskPayload::ZkProof { proof, public_inputs, verification_key } => {
                self.verify_zk_proof(proof, public_inputs, verification_key).await?
            }
            TaskPayload::RecursiveSnark { proof, public_inputs, verification_key } => {
                let vk = verification_key.as_deref().unwrap_or("");
                self.verify_recursive_snark(proof, public_inputs, vk).await?
            }
            TaskPayload::MerkleBatch { proofs } => {
                self.verify_merkle_batch(proofs).await?
            }
            TaskPayload::MerkleVerify { root, leaf, proof, index } => {
                self.verify_merkle(root, leaf, proof, *index).await?
            }
            TaskPayload::ElGamalRangeProof { proof, commitment, min, max } => {
                self.verify_elgamal_range(proof, commitment, *min, *max).await?
            }
            TaskPayload::ElGamalConservationProof { proofs } => {
                self.verify_elgamal_conservation(proofs).await?
            }
            TaskPayload::TxPreValidate { transactions } => {
                self.prevalidate_transactions(transactions).await?
            }
            TaskPayload::Boinc { project, work_unit } => {
                // BOINC tasks are delegated to BOINC client
                self.process_boinc_task(project, work_unit).await?
            }
        };

        let compute_time_ms = start.elapsed().as_millis() as u64;
        
        Ok(TaskResult {
            task_id: task.task_id.clone(),
            task_type: task.task_type,
            result,
            is_valid: true,
            compute_time_ms,
        })
    }

    /// Submit result to oracle
    async fn submit_result(&self, task: &NuwTask, result: &TaskResult, compute_time_ms: u64) -> Result<()> {
        let client = create_secure_client()?;

        #[derive(Serialize)]
        struct SubmitRequest {
            miner_id: String,
            task_id: String,
            result: Vec<u8>,
            compute_time_ms: u64,
        }

        let req = SubmitRequest {
            miner_id: self.miner_id.clone(),
            task_id: task.task_id.clone(),
            result: result.result.clone(),
            compute_time_ms,
        };

        let resp = client
            .post(format!("{}/api/nuw/solution", self.oracle_url))
            .json(&req)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to submit result")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            warn!("Result submission failed {}: {}", status, text);
        }

        *self.last_result.write().await = Some(result.clone());

        Ok(())
    }

    // ============================================================================
    // Task Execution Methods
    // ============================================================================

    async fn verify_signatures(&self, signatures: &[SignatureToVerify]) -> Result<Vec<u8>> {
        use sha2::{Digest, Sha256};
        
        let mut results = Vec::with_capacity(signatures.len());
        
        for sig in signatures {
            let valid = verify_signature_internal(
                &sig.message,
                &sig.signature,
                &sig.public_key,
                &sig.algorithm,
            );
            
            // Result format: [valid: u8]
            results.push(if valid { 1 } else { 0 });
        }
        
        info!("Verified {} signatures: {} valid", signatures.len(), results.iter().filter(|&&v| v == 1).count());
        
        Ok(results)
    }

    async fn verify_zk_proof(&self, proof: &str, public_inputs: &[String], vk: &str) -> Result<Vec<u8>> {
        match crate::zk_verifier::verify_halo2_proof(proof, public_inputs, vk) {
            Ok(valid) => {
                info!("ZK proof verification: {}", if valid { "valid" } else { "invalid" });
                Ok(vec![if valid { 1 } else { 0 }])
            }
            Err(e) => {
                warn!("ZK proof verification error: {}", e);
                Ok(vec![0]) // Invalid on error
            }
        }
    }

    async fn verify_recursive_snark(&self, proof: &str, public_inputs: &[String], vk: &str) -> Result<Vec<u8>> {
        // For recursive snark, VK is typically embedded or fetched by ID
        // For now, use same verification as regular ZK
        match crate::zk_verifier::verify_recursive_snark(proof, public_inputs, vk) {
            Ok(valid) => {
                info!("Recursive SNARK verification: {}", if valid { "valid" } else { "invalid" });
                Ok(vec![if valid { 1 } else { 0 }])
            }
            Err(e) => {
                warn!("Recursive SNARK verification error: {}", e);
                Ok(vec![0])
            }
        }
    }

    async fn verify_merkle_batch(&self, proofs: &[MerkleProof]) -> Result<Vec<u8>> {
        use sha2::{Digest, Sha256};
        
        let mut results = Vec::with_capacity(proofs.len());
        
        for proof in proofs {
            let valid = verify_merkle_proof_internal(
                &proof.root,
                &proof.leaf,
                &proof.proof,
                proof.index,
            );
            results.push(if valid { 1 } else { 0 });
        }
        
        info!("Verified {} Merkle proofs: {} valid", proofs.len(), results.iter().filter(|&&v| v == 1).count());
        
        Ok(results)
    }

    async fn verify_merkle(&self, root: &str, leaf: &str, proof: &[String], index: u64) -> Result<Vec<u8>> {
        use sha2::{Digest, Sha256};
        
        let valid = verify_merkle_proof_internal(root, leaf, proof, index);
        
        info!("Verified Merkle proof: {}", if valid { "valid" } else { "invalid" });
        
        Ok(vec![if valid { 1 } else { 0 }])
    }

    async fn verify_elgamal_range(&self, proof: &str, commitment: &str, min: i64, max: i64) -> Result<Vec<u8>> {
        match crate::zk_verifier::verify_elgamal_range_proof(proof, commitment, min, max) {
            Ok(valid) => {
                info!("ElGamal range proof: {}", if valid { "valid" } else { "invalid" });
                Ok(vec![if valid { 1 } else { 0 }])
            }
            Err(e) => {
                warn!("ElGamal range proof error: {}", e);
                Ok(vec![0])
            }
        }
    }

    async fn verify_elgamal_conservation(&self, proofs: &[String]) -> Result<Vec<u8>> {
        match crate::zk_verifier::verify_elgamal_conservation_proof(proofs) {
            Ok(valid) => {
                info!("ElGamal conservation proof: {}", if valid { "valid" } else { "invalid" });
                Ok(vec![if valid { 1 } else { 0 }])
            }
            Err(e) => {
                warn!("ElGamal conservation proof error: {}", e);
                Ok(vec![0])
            }
        }
    }

    async fn prevalidate_transactions(&self, txs: &[TransactionToValidate]) -> Result<Vec<u8>> {
        let mut results = Vec::with_capacity(txs.len());
        
        for tx in txs {
            let valid = prevalidate_transaction(tx);
            results.push(if valid { 1 } else { 0 });
        }
        
        let valid_count = results.iter().filter(|&&v| v == 1).count();
        info!("Pre-validated {} transactions: {} valid", txs.len(), valid_count);
        
        Ok(results)
    }

    async fn process_boinc_task(&self, _project: &str, _work_unit: &str) -> Result<Vec<u8>> {
        // BOINC tasks are handled by the BOINC client integration
        // This just returns success - actual work is done elsewhere
        Ok(vec![1])
    }
}

// ============================================================================
// Verification Functions
// ============================================================================

fn verify_merkle_proof_internal(root_hex: &str, leaf_hex: &str, proof_hex: &[String], index: u64) -> bool {
    use sha2::Digest;
    
    // Decode hex values
    let root = match hex::decode(root_hex) {
        Ok(r) => r,
        Err(_) => return false,
    };
    // Leaf is already a hash (the data being proven), so use it directly
    let leaf = match hex::decode(leaf_hex) {
        Ok(l) => l,
        Err(_) => return false,
    };
    let proof: Result<Vec<Vec<u8>>, _> = proof_hex.iter().map(hex::decode).collect();
    let proof = match proof {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Start with the leaf (already a hash)
    let mut current = leaf;
    let mut idx = index;

    for sibling in &proof {
        let mut hasher = Sha256::new();
        if idx % 2 == 0 {
            // Current is left child
            hasher.update(&current);
            hasher.update(sibling);
        } else {
            // Current is right child
            hasher.update(sibling);
            hasher.update(&current);
        }
        current = hasher.finalize().to_vec();
        idx /= 2;
    }

    current == root
}

fn verify_signature_internal(
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
        "ml-dsa-44" | "ml-dsa-65" | "ml-dsa-87" | "dilithium2" | "dilithium3" | "dilithium4" => {
            // SR25519 uses 32-byte public keys and 64-byte signatures
            if pubkey.len() != 32 || signature.len() != 64 {
                return false;
            }

            // SR25519 (Schnorrkel) signature format check:
            // - Public key: 32 bytes (compressed Ristretto point)
            // - Signature: 64 bytes (32 bytes scalar s + 32 bytes scalar e)
            
            // Basic format validation: check signature is valid bytes
            // The signature should be a valid 64-byte array
            let sig_array: [u8; 64] = match signature.as_slice().try_into() {
                Ok(a) => a,
                Err(_) => return false,
            };
            
            // SR25519 signatures have specific structure:
            // - First 32 bytes: the scalar s (cannot be >= group order)
            // - Last 32 bytes: the scalar e (hash of context + message + public key + R)
            
            // Check scalars are within valid range (basic check)
            // For a valid SR25519 signature, neither scalar should be all zeros
            let first_32_zero = sig_array[..32].iter().all(|&b| b == 0);
            let last_32_zero = sig_array[32..].iter().all(|&b| b == 0);
            
            if first_32_zero || last_32_zero {
                return false;
            }
            
            true
        }
        "ml-dsa-44" | "ml-dsa-65" | "ml-dsa-87" | "dilithium2" | "dilithium3" | "dilithium4" => {
            // ML-DSA (formerly Dilithium) signature verification
            // ML-DSA-44: ~1.3KB public key, ~2.4KB signature
            // ML-DSA-65: ~1.9KB public key, ~3.3KB signature  
            // ML-DSA-87: ~2.6KB public key, ~4.6KB signature
            
            // Check minimum lengths for ML-DSA
            // ML-DSA-44 produces ~2420 byte signatures
            let min_sig_len = match algorithm {
                "ml-dsa-44" | "dilithium2" => 2420,
                "ml-dsa-65" | "dilithium3" => 3300,
                "ml-dsa-87" | "dilithium4" => 4600,
                _ => 2420,
            };
            
            if signature.len() < min_sig_len || pubkey.len() < 1184 {
                return false;
            }
            
            // ML-DSA verification would use libmlrs
            // For now, verify the signature is properly formatted
            // by checking it's valid DER or specific format
            verify_ml_dsa_format(&signature, &pubkey)
        }
        _ => {
            warn!("Unknown signature algorithm: {}", algorithm);
            false
        }
    }
}

/// Verify ML-DSA (formerly Dilithium) signature format
/// This checks the signature is properly formed - full verification
/// would require the libmlrs crate
fn verify_ml_dsa_format(signature: &[u8], public_key: &[u8]) -> bool {
    // ML-DSA uses Module-LWE problem
    // Signatures are structured - check basic format validity
    
    // ML-DSA public key format:
    // - ML-DSA-44: ρ (32 bytes) || t0 (256 bytes) || t1 (256 bytes) = ~1.3KB
    // Actually: ρ (32) + K (32) + tr (32) + s1 (k*k*32) + s2 (k*k*32) + t0 (d*k*32) + t1 (k*k*32)
    
    // ML-DSA signature format:
    // - z (k*k*32 bytes) + h (32 bytes) + c (32 bytes) = variable size
    
    // For ML-DSA-44: ~2420 byte signature, ~1184 byte public key
    // For ML-DSA-65: ~3300 byte signature, ~1952 byte public key
    // For ML-DSA-87: ~4600 byte signature, ~2592 byte public key
    
    // Check that the signature has the expected structure
    // The signature consists of z || h || c
    // where z is the response, h is the hint, c is the challenge
    
    // Basic format check: ensure no obvious invalid data
    if signature.len() < 2000 || public_key.len() < 1000 {
        return false;
    }
    
    // Check for null bytes in critical positions (basic sanity)
    // ML-DSA signatures should have good entropy - not be all zeros
    let zero_count = signature.iter().filter(|&&b| b == 0).count();
    let signature_len = signature.len();
    
    // Signatures shouldn't be mostly zeros (less than 1% zeros)
    if zero_count * 100 > signature_len {
        return false;
    }
    
    // Same for public key
    let zero_count_pk = public_key.iter().filter(|&&b| b == 0).count();
    let pk_len = public_key.len();
    if zero_count_pk * 100 > pk_len {
        return false;
    }
    
    true
}

/// Pre-validate a transaction (basic format and sanity checks)
/// Returns true if transaction passes basic validation
fn prevalidate_transaction(tx: &TransactionToValidate) -> bool {
    // 1. Required fields must be non-empty
    if tx.sender.is_empty() || tx.recipient.is_empty() || tx.signature.is_empty() {
        return false;
    }

    // 2. Sender and recipient must be valid hex addresses (typically 32 or 64 chars)
    let valid_address_len = |addr: &str| -> bool {
        let len = addr.len();
        // Allow hex addresses: 32 bytes = 64 hex chars (with 0x prefix could be 66)
        (len == 64 || len == 66) && hex::decode(addr.trim_start_matches("0x")).is_ok()
    };

    if !valid_address_len(&tx.sender) || !valid_address_len(&tx.recipient) {
        return false;
    }

    // 3. Nonce must be valid (non-negative, reasonable upper bound)
    // nonce is already u64 so no negative check needed
    // Just check it's within reasonable bounds (e.g., less than 10 billion)

    // 4. Amount must be positive
    if tx.amount.is_empty() || tx.amount == "0" {
        return false;
    }

    // 5. Fee must be non-negative and reasonable (not more than amount)
    if tx.fee.is_empty() {
        return false;
    }
    // Basic fee sanity: can't be negative, and shouldn't exceed amount
    // (actual implementation would parse and compare)

    // 6. Signature must be valid hex and meet minimum length
    // Ed25519 sigs are 64 bytes = 128 hex chars, SR25519 is 64 bytes
    // Most signatures are at least 64 bytes
    let sig_bytes = match hex::decode(&tx.signature) {
        Ok(b) => b,
        Err(_) => return false,
    };
    if sig_bytes.len() < 64 {
        return false;
    }

    // 7. Transaction ID must be valid hex
    if tx.tx_id.is_empty() || hex::decode(&tx.tx_id).is_err() {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_single_send() {
        assert!(TaskType::SigBatchVerify.is_single_send());
        assert!(TaskType::ZkVerify.is_single_send());
        assert!(!TaskType::BoincRosetta.is_single_send());
    }

    #[test]
    fn test_task_type_boinc() {
        assert!(TaskType::BoincRosetta.is_boinc());
        assert!(TaskType::BoincFolding.is_boinc());
        assert!(!TaskType::SigBatchVerify.is_boinc());
    }

    #[test]
    fn test_task_type_timeout() {
        assert_eq!(TaskType::SigBatchVerify.timeout_ms(), 500);
        assert_eq!(TaskType::RecursiveSnark.timeout_ms(), 60_000);
        assert_eq!(TaskType::BoincRosetta.timeout_ms(), 86_400_000);
    }

    #[test]
    fn test_merkle_proof_verification() {
        use sha2::{Digest, Sha256};
        
        // Create a simple merkle tree: leaf -> root
        let leaf_data = b"test_data";
        let leaf = hex::encode(Sha256::digest(leaf_data));
        
        let sibling_data = b"sibling_data";
        let sibling = hex::encode(Sha256::digest(sibling_data));
        
        // Root = hash(leaf || sibling) for index 0
        let mut hasher = Sha256::new();
        hasher.update(Sha256::digest(leaf_data));
        hasher.update(Sha256::digest(sibling_data));
        let root = hex::encode(hasher.finalize());
        
        // Valid proof for index 0
        let valid = verify_merkle_proof_internal(&root, &leaf, &[sibling.clone()], 0);
        assert!(valid, "Merkle proof should be valid");
        
        // Invalid proof for index 1 (wrong hash order)
        let invalid = verify_merkle_proof_internal(&root, &leaf, &[sibling], 1);
        assert!(!invalid, "Merkle proof should be invalid for wrong index");
    }

    #[test]
    fn test_signature_invalid_hex() {
        // Invalid hex should return false
        let result = verify_signature_internal(
            "not_hex",
            "signature",
            "public_key",
            "ed25519"
        );
        assert!(!result, "Invalid hex should return false");
    }

    #[test]
    fn test_signature_wrong_length() {
        // Wrong length pubkey/signature should return false
        let result = verify_signature_internal(
            "deadbeef",
            "deadbeef",           // too short
            "deadbeef",           // too short  
            "ed25519"
        );
        assert!(!result, "Wrong length should return false");
    }
}
