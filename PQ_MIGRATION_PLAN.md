# Post-Quantum Cryptography Migration Plan - Miner Component

## Overview
This document outlines the necessary changes to migrate the `/miner` component from classical cryptography to a hybrid post-quantum cryptography system with algorithm agility, focusing on PoUW (Proof of Useful Work) operations and oracle interactions.

## Current State Analysis

### Current Cryptographic Dependencies
- **No explicit cryptographic dependencies** in Cargo.toml currently
- **Hash Functions**: SHA-3 (via `sha3` crate)
- **Random Number Generation**: OS-level randomness
- **FFI Integration**: BOINC and F@H client interactions
- **Oracle Communication**: HTTP-based with no explicit crypto

### Security-Critical Components
1. **Work Unit Authentication**: Verifying job authenticity from oracles
2. **Result Submission**: Proving work completion to oracles
3. **Miner Identity**: Establishing miner credentials and reputation
4. **Oracle Trust**: Validating oracle-signed challenges
5. **Anti-Fraud**: Preventing work result forgery or replay attacks

## Migration Strategy

### Phase 1: Foundation Infrastructure

#### 1.1 Dependencies Update
Add to `Cargo.toml`:
```toml
# Post-Quantum Cryptography
pqcrypto = "0.19"
pqcrypto-falcon = "0.6"
pqcrypto-dilithium = "0.5"
pqcrypto-kyber = "0.7"
pqcrypto-traits = "0.3"

# Lightweight symmetric crypto
ascon-aead = "0.4"
ascon-hash = "0.3"

# Classical crypto for hybrid mode
ed25519-dalek = { version = "2.2.0", features = ["std", "rand_core", "serde"] }
ring = "0.17"

# Enhanced security
zeroize = "1.7"  # Secure memory clearing
rand = { version = "0.8", features = ["std"] }

# Enhanced serialization for crypto types
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
```

#### 1.2 Shared Crypto Infrastructure
Create `src/crypto/mod.rs`:
```rust
// Re-export shared crypto types from node component
// This ensures consistency across components

pub use chert_node::crypto::*;

// Miner-specific crypto extensions
pub mod miner_identity;
pub mod work_authentication;
pub mod oracle_communication;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerCredentials {
    pub miner_id: String,
    pub keypairs: Vec<UniversalKeyPair>,
    pub supported_algorithms: Vec<SignatureAlgorithm>,
    pub reputation_score: f64,
    pub registration_timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkProof {
    pub work_unit_id: String,
    pub result_hash: String,
    pub computation_proof: ComputationProof,
    pub miner_signatures: Vec<UniversalSignature>,
    pub timestamp: u64,
    pub nonce: [u8; 32], // Anti-replay protection
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputationProof {
    BoincResult {
        result_file_hash: String,
        stderr_hash: String,
        execution_time: u64,
        resource_usage: ResourceUsage,
    },
    FoldingResult {
        trajectory_hash: String,
        protein_id: String,
        computation_time: u64,
        energy_calculation: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time: f64,
    pub wall_time: f64,
    pub memory_peak: u64,
    pub disk_io: u64,
}
```

#### 1.3 Miner Identity Management
Create `src/crypto/miner_identity.rs`:
```rust
use super::*;
use anyhow::{Result, anyhow};
use zeroize::ZeroizeOnDrop;
use std::path::Path;

#[derive(ZeroizeOnDrop)]
pub struct MinerIdentity {
    pub credentials: MinerCredentials,
    #[zeroize(skip)] // Don't zeroize public data
    pub public_credentials: MinerPublicCredentials,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerPublicCredentials {
    pub miner_id: String,
    pub public_keys: Vec<(SignatureAlgorithm, Vec<u8>)>,
    pub supported_algorithms: Vec<SignatureAlgorithm>,
    pub registration_signature: UniversalSignature,
}

impl MinerIdentity {
    pub fn generate(preferred_algorithms: &[SignatureAlgorithm]) -> Result<Self> {
        let miner_id = uuid::Uuid::new_v4().to_string();
        let mut keypairs = Vec::new();
        let mut public_keys = Vec::new();

        // Generate keypairs for each supported algorithm
        for algorithm in preferred_algorithms {
            let keypair = UniversalKeyPair::generate(algorithm.clone())?;
            public_keys.push((algorithm.clone(), keypair.public_key.clone()));
            keypairs.push(keypair);
        }

        let credentials = MinerCredentials {
            miner_id: miner_id.clone(),
            keypairs,
            supported_algorithms: preferred_algorithms.to_vec(),
            reputation_score: 0.0,
            registration_timestamp: chrono::Utc::now().timestamp() as u64,
        };

        // Sign the public credentials with the primary key
        let primary_keypair = &credentials.keypairs[0];
        let public_creds_message = bincode::serialize(&(
            &miner_id,
            &public_keys,
            &preferred_algorithms,
            credentials.registration_timestamp,
        ))?;

        let registration_signature = primary_keypair.sign(&public_creds_message)?;

        let public_credentials = MinerPublicCredentials {
            miner_id,
            public_keys,
            supported_algorithms: preferred_algorithms.to_vec(),
            registration_signature,
        };

        Ok(MinerIdentity {
            credentials,
            public_credentials,
        })
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        use std::fs;
        let data = fs::read(path)?;
        let credentials: MinerCredentials = bincode::deserialize(&data)?;
        
        // Reconstruct public credentials
        let public_keys: Vec<(SignatureAlgorithm, Vec<u8>)> = credentials.keypairs
            .iter()
            .map(|kp| (kp.algorithm.clone(), kp.public_key.clone()))
            .collect();

        let public_creds_message = bincode::serialize(&(
            &credentials.miner_id,
            &public_keys,
            &credentials.supported_algorithms,
            credentials.registration_timestamp,
        ))?;

        let registration_signature = credentials.keypairs[0].sign(&public_creds_message)?;

        let public_credentials = MinerPublicCredentials {
            miner_id: credentials.miner_id.clone(),
            public_keys,
            supported_algorithms: credentials.supported_algorithms.clone(),
            registration_signature,
        };

        Ok(MinerIdentity {
            credentials,
            public_credentials,
        })
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use std::fs;
        let data = bincode::serialize(&self.credentials)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn sign_work_proof(&self, proof: &WorkProof) -> Result<Vec<UniversalSignature>> {
        let message = bincode::serialize(proof)?;
        let mut signatures = Vec::new();

        // Sign with all available keypairs for maximum compatibility
        for keypair in &self.credentials.keypairs {
            let signature = keypair.sign(&message)?;
            signatures.push(signature);
        }

        Ok(signatures)
    }

    pub fn get_preferred_algorithm(&self) -> SignatureAlgorithm {
        // Prefer PQ algorithms over classical
        for algorithm in &self.credentials.supported_algorithms {
            match algorithm {
                SignatureAlgorithm::Falcon512 | SignatureAlgorithm::Falcon1024 => return algorithm.clone(),
                SignatureAlgorithm::Dilithium2 | SignatureAlgorithm::Dilithium3 => return algorithm.clone(),
                _ => continue,
            }
        }
        // Fallback to first available
        self.credentials.supported_algorithms[0].clone()
    }
}
```

#### 1.4 Oracle Communication Security
Create `src/crypto/oracle_communication.rs`:
```rust
use super::*;
use anyhow::{Result, anyhow};
use reqwest::Client;

pub struct SecureOracleClient {
    client: Client,
    miner_identity: Arc<MinerIdentity>,
    trusted_oracles: HashMap<String, Vec<UniversalKeyPair>>, // Oracle public keys
    session_keys: HashMap<String, NetworkSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureJobRequest {
    pub miner_public_credentials: MinerPublicCredentials,
    pub request_timestamp: u64,
    pub preferred_work_types: Vec<String>,
    pub hardware_capabilities: HardwareCapabilities,
    pub request_signature: UniversalSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureJobResponse {
    pub challenge: PouwChallenge,
    pub work_deadline: u64,
    pub oracle_signature: UniversalSignature,
    pub session_key: Option<Vec<u8>>, // For ongoing communication
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub gpu_available: bool,
    pub gpu_memory_gb: Option<f64>,
    pub supported_platforms: Vec<String>,
}

impl SecureOracleClient {
    pub fn new(
        miner_identity: Arc<MinerIdentity>,
        trusted_oracles: HashMap<String, Vec<UniversalKeyPair>>
    ) -> Self {
        SecureOracleClient {
            client: Client::new(),
            miner_identity,
            trusted_oracles,
            session_keys: HashMap::new(),
        }
    }

    pub async fn request_work(
        &mut self,
        oracle_url: &str,
        work_types: &[String],
        hardware_caps: HardwareCapabilities
    ) -> Result<SecureJobResponse> {
        // Create authenticated work request
        let request = SecureJobRequest {
            miner_public_credentials: self.miner_identity.public_credentials.clone(),
            request_timestamp: chrono::Utc::now().timestamp() as u64,
            preferred_work_types: work_types.to_vec(),
            hardware_capabilities: hardware_caps,
            request_signature: UniversalSignature {
                algorithm: SignatureAlgorithm::Ed25519, // Placeholder
                signature: vec![],
            },
        };

        // Sign the request
        let message = bincode::serialize(&(
            &request.miner_public_credentials,
            request.request_timestamp,
            &request.preferred_work_types,
            &request.hardware_capabilities,
        ))?;

        let signature = self.miner_identity.credentials.keypairs[0].sign(&message)?;
        let mut signed_request = request;
        signed_request.request_signature = signature;

        // Send request to oracle
        let response = self.client
            .post(&format!("{}/secure_work_request", oracle_url))
            .json(&signed_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Oracle request failed: {}", response.status()));
        }

        let job_response: SecureJobResponse = response.json().await?;

        // Verify oracle signature
        self.verify_oracle_response(oracle_url, &job_response)?;

        Ok(job_response)
    }

    pub async fn submit_work_result(
        &self,
        oracle_url: &str,
        work_proof: WorkProof
    ) -> Result<SubmissionResponse> {
        // Sign the work proof
        let mut signed_proof = work_proof;
        signed_proof.miner_signatures = self.miner_identity.sign_work_proof(&signed_proof)?;

        // Submit to oracle
        let response = self.client
            .post(&format!("{}/submit_work_result", oracle_url))
            .json(&signed_proof)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Work submission failed: {}", response.status()));
        }

        let submission_response: SubmissionResponse = response.json().await?;
        Ok(submission_response)
    }

    fn verify_oracle_response(&self, oracle_url: &str, response: &SecureJobResponse) -> Result<()> {
        // Get oracle public keys
        let oracle_keys = self.trusted_oracles.get(oracle_url)
            .ok_or_else(|| anyhow!("Unknown oracle: {}", oracle_url))?;

        // Verify challenge signature
        let message = bincode::serialize(&response.challenge)?;
        
        for oracle_key in oracle_keys {
            if oracle_key.verify(&message, &response.oracle_signature)? {
                return Ok(());
            }
        }

        Err(anyhow!("Invalid oracle signature"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResponse {
    pub accepted: bool,
    pub reward_points: Option<u64>,
    pub error_message: Option<String>,
    pub oracle_signature: UniversalSignature,
}
```

### Phase 2: Core Miner Updates

#### 2.1 Update Main Library (`lib.rs`)
Integrate cryptographic security into job processing:
```rust
pub mod crypto;

use crypto::{MinerIdentity, SecureOracleClient, WorkProof, ComputationProof, HardwareCapabilities};
use std::sync::Arc;
use anyhow::Result;

pub struct SecureMiner {
    identity: Arc<MinerIdentity>,
    oracle_client: SecureOracleClient,
    work_dir: PathBuf,
    cores_dir: PathBuf,
}

impl SecureMiner {
    pub fn new(
        identity_file: Option<&Path>,
        trusted_oracles: HashMap<String, Vec<UniversalKeyPair>>,
        work_dir: PathBuf,
        cores_dir: PathBuf,
    ) -> Result<Self> {
        let identity = match identity_file {
            Some(path) if path.exists() => {
                println!("📋 Loading existing miner identity from: {:?}", path);
                Arc::new(MinerIdentity::load_from_file(path)?)
            },
            Some(path) => {
                println!("🔑 Generating new miner identity...");
                let identity = MinerIdentity::generate(&[
                    SignatureAlgorithm::Falcon512,
                    SignatureAlgorithm::Dilithium2,
                    SignatureAlgorithm::Ed25519, // Fallback
                ])?;
                identity.save_to_file(path)?;
                println!("💾 Saved new identity to: {:?}", path);
                Arc::new(identity)
            },
            None => {
                println!("🔑 Generating temporary miner identity...");
                Arc::new(MinerIdentity::generate(&[
                    SignatureAlgorithm::Falcon512,
                    SignatureAlgorithm::Ed25519, // Fallback
                ])?)
            }
        };

        let oracle_client = SecureOracleClient::new(identity.clone(), trusted_oracles);

        Ok(SecureMiner {
            identity,
            oracle_client,
            work_dir,
            cores_dir,
        })
    }

    pub async fn run_secure_job(&mut self, oracle_url: &str) -> Result<()> {
        // Get hardware capabilities
        let hardware_caps = HardwareCapabilities {
            cpu_cores: num_cpus::get() as u32,
            memory_gb: self.get_memory_gb(),
            gpu_available: self.detect_gpu(),
            gpu_memory_gb: self.get_gpu_memory_gb(),
            supported_platforms: vec!["linux".to_string(), "boinc".to_string(), "fah".to_string()],
        };

        // Request work from oracle
        let job_response = self.oracle_client.request_work(
            oracle_url,
            &["boinc".to_string(), "fah".to_string()],
            hardware_caps
        ).await?;

        println!("✅ Received secure work: {}", job_response.challenge.challenge_id);

        // Execute the work based on type
        let computation_proof = match job_response.challenge.work_type.as_str() {
            "boinc" => self.execute_boinc_work(&job_response.challenge).await?,
            "fah" => self.execute_fah_work(&job_response.challenge).await?,
            _ => return Err(anyhow::anyhow!("Unsupported work type")),
        };

        // Create work proof
        let work_proof = WorkProof {
            work_unit_id: job_response.challenge.challenge_id.clone(),
            result_hash: self.compute_result_hash(&computation_proof),
            computation_proof,
            miner_signatures: vec![], // Will be filled by oracle client
            timestamp: chrono::Utc::now().timestamp() as u64,
            nonce: rand::random(),
        };

        // Submit result
        let submission_response = self.oracle_client.submit_work_result(oracle_url, work_proof).await?;

        if submission_response.accepted {
            println!("🎉 Work accepted! Reward points: {:?}", submission_response.reward_points);
        } else {
            println!("❌ Work rejected: {:?}", submission_response.error_message);
        }

        Ok(())
    }

    async fn execute_boinc_work(&self, challenge: &PouwChallenge) -> Result<ComputationProof> {
        // Download and verify work unit
        let work_unit = self.download_and_verify_work_unit(challenge).await?;
        
        // Execute BOINC computation
        let start_time = std::time::Instant::now();
        let result = run_boinc_job(&work_unit)?;
        let execution_time = start_time.elapsed();

        // Hash the result files for verification
        let result_file_hash = self.hash_file(&result.output_file)?;
        let stderr_hash = self.hash_file(&result.stderr_file)?;

        Ok(ComputationProof::BoincResult {
            result_file_hash,
            stderr_hash,
            execution_time: execution_time.as_secs(),
            resource_usage: ResourceUsage {
                cpu_time: result.cpu_time,
                wall_time: execution_time.as_secs_f64(),
                memory_peak: result.memory_usage,
                disk_io: result.disk_io,
            },
        })
    }

    async fn execute_fah_work(&self, challenge: &PouwChallenge) -> Result<ComputationProof> {
        // Similar to BOINC but for Folding@Home
        // Implementation details...
        unimplemented!("F@H secure execution")
    }

    fn compute_result_hash(&self, proof: &ComputationProof) -> String {
        use ascon_hash::AsconHash;
        
        let serialized = bincode::serialize(proof).unwrap();
        let hash = AsconHash::digest(&serialized);
        hex::encode(hash)
    }
}

// Legacy function wrappers for backwards compatibility
pub async fn run_oracle_job(oracle_url: &str) -> Result<()> {
    println!("⚠️  Warning: Using legacy run_oracle_job. Consider upgrading to SecureMiner.");
    
    // Use temporary identity for legacy calls
    let temp_identity_dir = std::env::temp_dir().join("chert_miner_temp_identity");
    std::fs::create_dir_all(&temp_identity_dir)?;
    
    let mut secure_miner = SecureMiner::new(
        Some(&temp_identity_dir.join("identity.bin")),
        HashMap::new(), // Empty trusted oracles - legacy mode
        PathBuf::from("./work"),
        PathBuf::from("./cores"),
    )?;
    
    secure_miner.run_secure_job(oracle_url).await
}
```

#### 2.2 Update Oracle Module (`oracle.rs`)
Add cryptographic verification to oracle interactions:
```rust
use crate::crypto::*;
use anyhow::Result;

// Enhanced job structure with cryptographic proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureJob {
    pub task_id: String,
    pub work_type: String,
    pub input_data_url: String,
    pub input_data_hash: String,
    pub expected_runtime: u64,
    pub deadline: u64,
    pub oracle_signature: UniversalSignature,
    pub verification_data: VerificationData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationData {
    pub app_binary_hash: String,
    pub input_data_size: u64,
    pub expected_output_pattern: String,
    pub anti_replay_nonce: [u8; 32],
}

pub async fn fetch_secure_job(
    oracle_url: &str,
    miner_credentials: &MinerPublicCredentials,
    trusted_oracle_keys: &[UniversalKeyPair]
) -> Result<SecureJob> {
    let client = reqwest::Client::new();
    
    // Create authenticated request
    let request = JobRequest {
        miner_id: miner_credentials.miner_id.clone(),
        miner_public_keys: miner_credentials.public_keys.clone(),
        timestamp: chrono::Utc::now().timestamp() as u64,
        nonce: rand::random(),
    };

    let response = client
        .post(&format!("{}/api/v2/secure_job", oracle_url))
        .json(&request)
        .send()
        .await?;

    let secure_job: SecureJob = response.json().await?;

    // Verify oracle signature
    let message = bincode::serialize(&(
        &secure_job.task_id,
        &secure_job.work_type,
        &secure_job.input_data_hash,
        secure_job.deadline,
        &secure_job.verification_data,
    ))?;

    let mut signature_valid = false;
    for oracle_key in trusted_oracle_keys {
        if oracle_key.verify(&message, &secure_job.oracle_signature)? {
            signature_valid = true;
            break;
        }
    }

    if !signature_valid {
        return Err(anyhow::anyhow!("Invalid oracle signature on job"));
    }

    Ok(secure_job)
}

pub async fn submit_secure_result(
    oracle_url: &str,
    work_proof: &WorkProof,
) -> Result<SubmissionResponse> {
    let client = reqwest::Client::new();
    
    let response = client
        .post(&format!("{}/api/v2/submit_result", oracle_url))
        .json(work_proof)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Submission failed: {}", response.status()));
    }

    let submission_response: SubmissionResponse = response.json().await?;
    Ok(submission_response)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobRequest {
    miner_id: String,
    miner_public_keys: Vec<(SignatureAlgorithm, Vec<u8>)>,
    timestamp: u64,
    nonce: [u8; 32],
}

// Legacy functions for backwards compatibility
pub async fn fetch_job(oracle_url: &str) -> Result<SecureJob> {
    println!("⚠️  Warning: Using legacy fetch_job. Consider using fetch_secure_job.");
    
    // Convert legacy response to secure format (best effort)
    let client = reqwest::Client::new();
    let response = client.get(&format!("{}/api/job", oracle_url)).send().await?;
    
    // Convert legacy job to secure job with empty verification
    let legacy_job: serde_json::Value = response.json().await?;
    
    Ok(SecureJob {
        task_id: legacy_job["task_id"].as_str().unwrap_or("unknown").to_string(),
        work_type: legacy_job["work_type"].as_str().unwrap_or("boinc").to_string(),
        input_data_url: legacy_job["input_url"].as_str().unwrap_or("").to_string(),
        input_data_hash: "legacy_mode_no_verification".to_string(),
        expected_runtime: 3600,
        deadline: (chrono::Utc::now().timestamp() + 7200) as u64,
        oracle_signature: UniversalSignature {
            algorithm: SignatureAlgorithm::Ed25519,
            signature: vec![], // Empty for legacy mode
        },
        verification_data: VerificationData {
            app_binary_hash: "legacy_mode_no_verification".to_string(),
            input_data_size: 0,
            expected_output_pattern: ".*".to_string(),
            anti_replay_nonce: [0u8; 32],
        },
    })
}
```

### Phase 3: Integration and Testing

#### 3.1 Configuration Management
Create `src/config.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::crypto::SignatureAlgorithm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    pub identity_file: PathBuf,
    pub work_directory: PathBuf,
    pub cores_directory: PathBuf,
    pub preferred_algorithms: Vec<SignatureAlgorithm>,
    pub trusted_oracles: Vec<TrustedOracle>,
    pub security_settings: SecuritySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedOracle {
    pub url: String,
    pub public_keys: Vec<(SignatureAlgorithm, String)>, // Hex-encoded public keys
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    FullyTrusted,
    Conditional,
    TestOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub require_oracle_signatures: bool,
    pub allow_classical_fallback: bool,
    pub max_work_duration: u64,
    pub require_result_verification: bool,
    pub enable_anti_replay: bool,
}

impl Default for MinerConfig {
    fn default() -> Self {
        MinerConfig {
            identity_file: PathBuf::from("./miner_identity.bin"),
            work_directory: PathBuf::from("./work"),
            cores_directory: PathBuf::from("./cores"),
            preferred_algorithms: vec![
                SignatureAlgorithm::Falcon512,
                SignatureAlgorithm::Dilithium2,
                SignatureAlgorithm::Ed25519,
            ],
            trusted_oracles: vec![],
            security_settings: SecuritySettings {
                require_oracle_signatures: true,
                allow_classical_fallback: true,
                max_work_duration: 86400, // 24 hours
                require_result_verification: true,
                enable_anti_replay: true,
            },
        }
    }
}
```

#### 3.2 Update Main Binary (`main.rs`)
Integrate secure miner with command-line interface:
```rust
use miner::{SecureMiner, MinerConfig, crypto::*};
use std::path::PathBuf;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::init();

    let config_file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "miner_config.toml".to_string());

    let config = load_config(&config_file).unwrap_or_else(|_| {
        println!("⚠️  Config file not found, using defaults");
        MinerConfig::default()
    });

    // Initialize secure miner
    let mut secure_miner = SecureMiner::new(
        Some(&config.identity_file),
        convert_trusted_oracles(&config.trusted_oracles),
        config.work_directory,
        config.cores_directory,
    )?;

    println!("🚀 Chert Secure Miner started!");
    println!("🔐 Supported algorithms: {:?}", config.preferred_algorithms);

    // Main mining loop
    loop {
        for oracle in &config.trusted_oracles {
            if let Err(e) = secure_miner.run_secure_job(&oracle.url).await {
                eprintln!("❌ Error processing job from {}: {}", oracle.url, e);
            }
            
            // Wait between job requests
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }
}

fn load_config(config_file: &str) -> Result<MinerConfig> {
    let content = std::fs::read_to_string(config_file)?;
    let config: MinerConfig = toml::from_str(&content)?;
    Ok(config)
}

fn convert_trusted_oracles(
    oracles: &[TrustedOracle]
) -> HashMap<String, Vec<UniversalKeyPair>> {
    let mut result = HashMap::new();
    
    for oracle in oracles {
        let mut keys = Vec::new();
        for (algorithm, hex_key) in &oracle.public_keys {
            if let Ok(key_bytes) = hex::decode(hex_key) {
                let keypair = UniversalKeyPair {
                    algorithm: algorithm.clone(),
                    public_key: key_bytes,
                    private_key: vec![], // Public key only
                };
                keys.push(keypair);
            }
        }
        result.insert(oracle.url.clone(), keys);
    }
    
    result
}
```

## Migration Timeline

### Week 1-2: Foundation
- [ ] Add PQ crypto dependencies
- [ ] Implement miner identity system
- [ ] Create secure oracle communication
- [ ] Set up configuration management

### Week 3-4: Core Integration
- [ ] Update job request/response flows
- [ ] Implement work proof generation
- [ ] Add cryptographic verification
- [ ] Test with local oracle

### Week 5-6: Advanced Features
- [ ] Implement anti-replay protection
- [ ] Add hardware capability detection
- [ ] Optimize signature operations
- [ ] Add comprehensive logging

### Week 7-8: Testing & Hardening
- [ ] Security testing and auditing
- [ ] Performance optimization
- [ ] Integration with node/poi components
- [ ] Documentation and deployment

## Security Considerations

### Anti-Fraud Measures
- **Work Uniqueness**: Each work unit has unique nonce to prevent replay
- **Result Verification**: Hash-based verification of computation outputs
- **Timeline Verification**: Enforce realistic computation times
- **Resource Attestation**: Prove actual resource usage during computation

### Key Management
- **Identity Persistence**: Secure storage of miner credentials
- **Key Rotation**: Support for periodic key updates
- **Emergency Recovery**: Backup and recovery procedures
- **Hardware Security**: Optional HSM integration

### Network Security
- **Oracle Authentication**: Verify oracle identity before accepting work
- **Session Security**: Use PQ key exchange for ongoing communications
- **Traffic Analysis Resistance**: Randomize communication patterns
- **DoS Protection**: Rate limiting and resource management

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_miner_identity_generation() {
        let identity = MinerIdentity::generate(&[
            SignatureAlgorithm::Falcon512,
            SignatureAlgorithm::Ed25519,
        ]).unwrap();
        
        assert_eq!(identity.credentials.keypairs.len(), 2);
        assert!(identity.credentials.miner_id.len() > 0);
    }

    #[tokio::test]
    async fn test_work_proof_signing() {
        let identity = MinerIdentity::generate(&[SignatureAlgorithm::Falcon512]).unwrap();
        
        let proof = WorkProof {
            work_unit_id: "test".to_string(),
            result_hash: "hash".to_string(),
            computation_proof: ComputationProof::BoincResult {
                result_file_hash: "result".to_string(),
                stderr_hash: "stderr".to_string(),
                execution_time: 100,
                resource_usage: ResourceUsage {
                    cpu_time: 100.0,
                    wall_time: 110.0,
                    memory_peak: 1024,
                    disk_io: 512,
                },
            },
            miner_signatures: vec![],
            timestamp: chrono::Utc::now().timestamp() as u64,
            nonce: [0u8; 32],
        };

        let signatures = identity.sign_work_proof(&proof).unwrap();
        assert_eq!(signatures.len(), 1);
    }

    #[test]
    fn test_config_serialization() {
        let config = MinerConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: MinerConfig = toml::from_str(&serialized).unwrap();
        
        assert_eq!(config.preferred_algorithms, deserialized.preferred_algorithms);
    }
}
```

### Integration Tests
- Test with mock oracle endpoints
- Verify end-to-end job processing
- Test network failure scenarios
- Validate signature verification chains

## Performance Considerations

### Optimization Targets
- **Signature Generation**: <100ms for Falcon512
- **Verification**: <50ms for all algorithms
- **Memory Usage**: <10MB additional overhead
- **Network Overhead**: <5% additional bandwidth

### Resource Management
- Limit concurrent work units
- Implement graceful degradation under load
- Optimize crypto operations for available hardware
- Cache verification results where appropriate

## Deployment and Rollback

### Deployment Strategy
1. **Parallel Deployment**: Run old and new miners side-by-side
2. **Gradual Migration**: Move oracles to secure endpoints incrementally
3. **Monitoring**: Track signature verification success rates
4. **Validation**: Compare output quality between versions

### Rollback Procedures
- Maintain legacy job processing capability
- Implement automatic fallback to classical crypto
- Monitor for PQ algorithm failures or attacks
- Quick disable mechanism for emergency situations

---

**Next Steps**: Coordinate with node and poi components to ensure compatible PQ implementations and shared crypto infrastructure.
