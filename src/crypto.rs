/// Cryptographic utilities for the Chert miner
///
/// This module extends the shared silica-models crypto with miner-specific
/// functionality while maintaining consistency with the ecosystem.
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use silica_models::crypto::{ChertCrypto, ChertHash, HashAlgorithm, StandardCrypto, domains};

/// Miner-specific cryptographic constants
pub mod constants {
    /// Domain separator for BOINC binary verification
    pub const BOINC_BINARY_DOMAIN: &[u8] = b"CHERT_MINER_BOINC_BINARY_V1";

    /// Domain separator for work result hashing
    pub const WORK_RESULT_DOMAIN: &[u8] = b"CHERT_MINER_WORK_RESULT_V1";

    /// Expected hash length in hex chars (64 for SHA256)
    pub const SHA256_HEX_LENGTH: usize = 64;
}

/// Miner-specific hash utilities that extend the shared crypto
pub struct MinerHashUtils;

impl MinerHashUtils {
    /// Calculate file hash for integrity verification using shared crypto
    pub fn hash_file_content(data: &[u8]) -> String {
        let hash = StandardCrypto::hash_with_domain(
            HashAlgorithm::Sha256,
            Some(domains::FILE_VERIFICATION),
            data,
        );
        hash.hex
    }

    /// Calculate BOINC binary hash for security verification
    pub fn hash_boinc_binary(data: &[u8]) -> String {
        let hash = StandardCrypto::hash_with_domain(
            HashAlgorithm::Sha256,
            Some(constants::BOINC_BINARY_DOMAIN),
            data,
        );
        hash.hex
    }

    /// Calculate work result hash using Blake3 for performance
    pub fn hash_work_result(data: &[u8]) -> String {
        let hash = StandardCrypto::hash_with_domain(
            HashAlgorithm::Blake3,
            Some(constants::WORK_RESULT_DOMAIN),
            data,
        );
        hash.hex
    }

    /// Verify downloaded file integrity with constant-time comparison
    /// Uses plain SHA256 (no domain separator) for compatibility with external hashes
    /// like those from GitHub releases
    pub fn verify_file_integrity(expected_hex: &str, data: &[u8]) -> Result<bool> {
        if expected_hex.len() != constants::SHA256_HEX_LENGTH {
            return Err(anyhow::anyhow!(
                "Invalid SHA256 hex length: expected {}, got {}",
                constants::SHA256_HEX_LENGTH,
                expected_hex.len()
            ));
        }

        // Use plain SHA256 without domain separator for external hash compatibility
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual_digest = hasher.finalize();

        let expected_bytes = hex::decode(expected_hex).context("Failed to decode expected hash")?;

        // Constant-time comparison to prevent timing attacks
        use subtle::ConstantTimeEq;
        Ok(actual_digest[..].ct_eq(expected_bytes.as_slice()).into())
    }

    /// Verify file integrity using domain-separated hash (for internal Chert files)
    pub fn verify_file_integrity_with_domain(expected_hex: &str, data: &[u8]) -> Result<bool> {
        if expected_hex.len() != constants::SHA256_HEX_LENGTH {
            return Err(anyhow::anyhow!(
                "Invalid SHA256 hex length: expected {}, got {}",
                constants::SHA256_HEX_LENGTH,
                expected_hex.len()
            ));
        }

        let actual_hash = StandardCrypto::hash_with_domain(
            HashAlgorithm::Sha256,
            Some(domains::FILE_VERIFICATION),
            data,
        );

        let expected_bytes = hex::decode(expected_hex).context("Failed to decode expected hash")?;

        let expected_hash = ChertHash {
            algorithm: HashAlgorithm::Sha256,
            digest: expected_bytes,
            hex: expected_hex.to_string(),
        };

        Ok(StandardCrypto::verify_hash(&expected_hash, &actual_hash))
    }

    /// Calculate plain SHA256 hash (for external compatibility)
    pub fn hash_plain_sha256(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Generate secure random bytes for nonces, keys, etc.
    pub fn generate_secure_random(length: usize) -> Result<Vec<u8>> {
        StandardCrypto::secure_random(length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_hash_consistency() {
        let data = b"test file content";
        let hash1 = MinerHashUtils::hash_file_content(data);
        let hash2 = MinerHashUtils::hash_file_content(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_plain_sha256_matches_external() {
        // Test that our plain SHA256 matches standard tools
        let data = b"hello world";
        let hash = MinerHashUtils::hash_plain_sha256(data);
        // Known SHA256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_verification_works_plain_sha256() {
        let data = b"test data for verification";
        let hash = MinerHashUtils::hash_plain_sha256(data);
        let result = MinerHashUtils::verify_file_integrity(&hash, data);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verification_fails_wrong_data() {
        let data1 = b"original data";
        let data2 = b"modified data";
        let hash = MinerHashUtils::hash_plain_sha256(data1);
        let result = MinerHashUtils::verify_file_integrity(&hash, data2);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_boinc_vs_file_hash_different() {
        let data = b"same data";
        let file_hash = MinerHashUtils::hash_file_content(data);
        let boinc_hash = MinerHashUtils::hash_boinc_binary(data);
        // Should be different due to different domain separators
        assert_ne!(file_hash, boinc_hash);
    }

    #[test]
    fn test_secure_random() {
        let bytes1 = MinerHashUtils::generate_secure_random(32).unwrap();
        let bytes2 = MinerHashUtils::generate_secure_random(32).unwrap();
        assert_eq!(bytes1.len(), 32);
        assert_eq!(bytes2.len(), 32);
        assert_ne!(bytes1, bytes2);
    }

    #[test]
    fn test_plain_vs_domain_separated_hash_differ() {
        let data = b"test data";
        let plain = MinerHashUtils::hash_plain_sha256(data);
        let domain_separated = MinerHashUtils::hash_file_content(data);
        // Plain SHA256 should differ from domain-separated hash
        assert_ne!(plain, domain_separated);
    }
}
