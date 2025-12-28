use super::BoincAutomation;
use crate::crypto::MinerHashUtils;
/// BOINC installation and download management module
/// Handles downloading, verifying, and installing BOINC client binaries
use anyhow::{Context, Result};
use tracing::{info, warn};

/// BOINC 8.2.8 verified SHA256 hashes from GitHub releases API
/// These hashes are pulled directly from GitHub's `digest` field in release assets
/// Last updated: 2025-12-04
/// Source: https://api.github.com/repos/BOINC/boinc/releases/tags/client_release/8.2/8.2.8
#[allow(dead_code)]
mod boinc_hashes {
    // Linux amd64 (x86_64) - Client packages (.deb)
    pub const LINUX_AMD64_NOBLE: &str =
        "c1be6726a5e84733fefb000dc5d467beae17ad621e485c6bafdb1e827b7ff882"; // Ubuntu 24.04 Noble
    pub const LINUX_AMD64_JAMMY: &str =
        "35841588b962db3481ec1453e2fd904c123c4e60919c5aaf5def0e8694d057a8"; // Ubuntu 22.04 Jammy
    pub const LINUX_AMD64_FOCAL: &str =
        "95b20a7cd4470b06b17adc709b995b237f4db8b577fe3fc2b00c95f8548e5fd8"; // Ubuntu 20.04 Focal
    pub const LINUX_AMD64_BOOKWORM: &str =
        "fe9c6a7b003bd4b8aec3bd3343ad24d3d662f8489bfa09795c241f8b263b5100"; // Debian 12 Bookworm
    pub const LINUX_AMD64_BULLSEYE: &str =
        "0db3f4963b6eb829ed6fe81204eb32b233a99a77d6c71127ae65c9a60cdab257"; // Debian 11 Bullseye
    pub const LINUX_AMD64_BUSTER: &str =
        "46951be38cabf708bc4c527b1382d61886304490a64add8a2a0390c63a50c4ad"; // Debian 10 Buster
    pub const LINUX_AMD64_TRIXIE: &str =
        "6642cc9b5ad1b7455474fe49da8d15d04c98f28d4e2ba1d05ef5744fff5522de"; // Debian 13 Trixie

    // Linux arm64 (aarch64) - Client packages (.deb)
    pub const LINUX_ARM64_NOBLE: &str =
        "59651387b6d2102ac19ce1577a3d9a1396625f5804c0b2f7a868adbffc6a1037"; // Ubuntu 24.04 Noble
    pub const LINUX_ARM64_JAMMY: &str =
        "de79f90955c1fd740b98bbd0fcdbdbc5d22b3fb3ac4b44ee0d9a40c78b430f20"; // Ubuntu 22.04 Jammy
    pub const LINUX_ARM64_FOCAL: &str =
        "ca80488e53bd87714ad9214d9e8a11ec7bb09b3d83e66611738abfe21fa5e1b5"; // Ubuntu 20.04 Focal
    pub const LINUX_ARM64_BOOKWORM: &str =
        "259a3483d71176103e60f45a96879fc61bddde11bfe87a663afa1dd1427c336f"; // Debian 12 Bookworm
    pub const LINUX_ARM64_BULLSEYE: &str =
        "232f88e577a708d1a03583a791f5a7961a05861f8718259ce04619ebf494374d"; // Debian 11 Bullseye
    pub const LINUX_ARM64_BUSTER: &str =
        "d762ffb6996a74ef531a2664c0b67a7d1335956f9b5fe073c4d9443b8c65f85c"; // Debian 10 Buster
    pub const LINUX_ARM64_TRIXIE: &str =
        "e1d0433f17e395be8ad774bf17b95ec3e89a1b399f9977171f50e71ead60f630"; // Debian 13 Trixie

    // Windows x86_64 - Installer and ZIP
    pub const WINDOWS_X64_EXE: &str =
        "ad2063b1dd854c2bc085117345713363887fc9848ea9c6d9b7299dada84097ee"; // .exe installer
    pub const WINDOWS_X64_ZIP: &str =
        "11c5fd5f8992fcf323a151ae60e54b16f42aa353956c691f376e9c4f3282560c"; // .zip archive

    // Windows ARM64
    pub const WINDOWS_ARM64_EXE: &str =
        "59cf1057201a2b176dc448484e50585cb7158ad1a928ffd818cf1239669c807a"; // .exe installer
    pub const WINDOWS_ARM64_ZIP: &str =
        "1f1f79ff828735f0d03b061642cff811e9b2908d63e8a3171c99fa9565e0a95c"; // .zip archive

    // Android APK
    pub const ANDROID_APK: &str =
        "fa7f2d53b067af4954dfe84e744080f5a9d7d2a8d26e20dd73172fc78392c670"; // Universal APK
    pub const ANDROID_ARMV6_APK: &str =
        "c63ed807fb5024ab327b1d8f212af26943f6fa012f90ead9271d078f44da3db4"; // ARMv6 APK

    // Linux RPM packages - x86_64 (Fedora)
    pub const LINUX_X64_FC37_RPM: &str =
        "a07aae460969b64f4bc245daf688cb2c7ab0b6153be32254ab0ae1092569391d";
    pub const LINUX_X64_FC38_RPM: &str =
        "eeebd53ac13af5081f69a6926a4c4e4dc3d430f54c345e3aa8b3ff95fe5ab435";
    pub const LINUX_X64_FC39_RPM: &str =
        "23f06919089ec6871124b294a778be29456a2d5a50e182e6b5bb560f752c65cc";
    pub const LINUX_X64_FC40_RPM: &str =
        "50d143f4fb2c19a5161492c5dbc8768b3beab02a10385401f087bbd54ca7afce";
    pub const LINUX_X64_FC41_RPM: &str =
        "e5411b2551e180894f932d5caec169b7bc056b7c04da6570b8486cf6e59e5cae";
    pub const LINUX_X64_FC42_RPM: &str =
        "2e5711ce8cd00c314d19681f5e1b928f49a52f33c4245e2884ffb84dd430036d";

    // Linux RPM packages - aarch64 (Fedora)
    pub const LINUX_ARM64_FC37_RPM: &str =
        "27dd59ba2327782bf070e9d8ea14cd4c6a6b732e706d1dbc72e57dd8761a634b";
    pub const LINUX_ARM64_FC38_RPM: &str =
        "5fb37eaada068d74aaa5492f0af42c6452c61b8b7bcea63135d5eb0d57d820bd";
    pub const LINUX_ARM64_FC39_RPM: &str =
        "a77ae3ed1b5864ebd76267949c145102cd5a971201b28428903eefe000340612";
    pub const LINUX_ARM64_FC40_RPM: &str =
        "09a2fa1f53cf12d10689cfd870b165609145daf4fa4be49aac8ff879d3e4ee4a";
    pub const LINUX_ARM64_FC41_RPM: &str =
        "f4f89ba29663470ae979c43cb5ab7d4c566c688648d75de5f5b447821c0c3108";
    pub const LINUX_ARM64_FC42_RPM: &str =
        "2e368334fefd006e8afa3d537a73163577dbf3b6e114590a2f973f50475ef80c";

    // OpenSUSE RPMs - x86_64
    pub const LINUX_X64_SUSE15_4_RPM: &str =
        "4e09bc98d0bc122b41482c5394496a4470a3c756f33372d032f5b1ec8544cb54";
    pub const LINUX_X64_SUSE15_5_RPM: &str =
        "435f1ec4a5499ea496f867adb0d210ce8fab8e0c754f62f67f438bea2a465a5f";
    pub const LINUX_X64_SUSE15_6_RPM: &str =
        "ca5d0589a31dd02dc3f11f1231ad7d726dfb005b7235cb02f44600f70ae42b92";
    pub const LINUX_X64_SUSE16_0_RPM: &str =
        "f6a1ebccc4ca4339a048ae225bc22dd16e48694ee19b3e362f3be8799fe6745e";

    // OpenSUSE RPMs - aarch64
    pub const LINUX_ARM64_SUSE15_4_RPM: &str =
        "f0e59a0d14f28f759cad5ec9c158864604187f6f370ec9185bdc74f004913e5e";
    pub const LINUX_ARM64_SUSE15_5_RPM: &str =
        "81f09f1d9a5042ddb2ef72755bc24130506a7008ddff294ab2adef7d04859010";
    pub const LINUX_ARM64_SUSE15_6_RPM: &str =
        "820b805df8b6c78f56f4b49d7248abda9d88173ed74078fdd303481edf44204d";
    pub const LINUX_ARM64_SUSE16_0_RPM: &str =
        "82ceff6c8a163174adc860e6266a01429f9fdf429733b1df3520e0e2f973091d";
}

/// BOINC version constant
const BOINC_VERSION: &str = "8.2.8";
const BOINC_BUILD: &str = "3429";

impl BoincAutomation {
    /// Auto-detect platform and download appropriate BOINC client
    pub async fn auto_install_boinc(&self) -> Result<()> {
        if self.is_boinc_installed() {
            info!("BOINC already installed, skipping download");
            return Ok(());
        }

        info!("BOINC not found, downloading and installing...");

        // Detect platform and choose appropriate download URL
        let (url, expected_sha256) = self.get_boinc_download_info()?;

        self.download_and_install(&url, expected_sha256).await?;
        info!("BOINC installation completed");
        Ok(())
    }

    /// Get platform-specific BOINC download information with verified hashes
    fn get_boinc_download_info(&self) -> Result<(String, Option<String>)> {
        use std::env;

        // Allow override via environment variables for testing or custom versions
        if let Ok(custom_url) = env::var("CHERT_BOINC_DOWNLOAD_URL") {
            if let Ok(env_hash) = env::var("CHERT_BOINC_EXPECTED_HASH") {
                if env_hash.len() == 64 && env_hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    info!("Using custom BOINC download URL: {}", custom_url);
                    return Ok((custom_url, Some(env_hash)));
                }
                return Err(anyhow::anyhow!(
                    "Invalid environment hash format: {}",
                    env_hash
                ));
            }
            return Err(anyhow::anyhow!(
                "Custom BOINC URL requires CHERT_BOINC_EXPECTED_HASH to be set"
            ));
        }

        // Detect platform and select appropriate download
        let (url, hash) = self.detect_platform_download()?;

        info!(
            "Using verified BOINC {} for detected platform",
            BOINC_VERSION
        );
        Ok((url, Some(hash.to_string())))
    }

    /// Detect platform and return appropriate BOINC download URL and hash
    fn detect_platform_download(&self) -> Result<(String, &'static str)> {
        let base_url = format!(
            "https://github.com/BOINC/boinc/releases/download/client_release/8.2/{}/",
            BOINC_VERSION
        );

        // Detect OS and architecture
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            // Try to detect Linux distribution
            let distro = self.detect_linux_distro();

            let (filename, hash) = match distro.as_str() {
                "ubuntu-24.04" | "noble" => (
                    format!(
                        "boinc-client_{}-{}_amd64_noble.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_AMD64_NOBLE,
                ),
                "ubuntu-22.04" | "jammy" => (
                    format!(
                        "boinc-client_{}-{}_amd64_jammy.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_AMD64_JAMMY,
                ),
                "ubuntu-20.04" | "focal" => (
                    format!(
                        "boinc-client_{}-{}_amd64_focal.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_AMD64_FOCAL,
                ),
                "debian-12" | "bookworm" => (
                    format!(
                        "boinc-client_{}-{}_amd64_bookworm.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_AMD64_BOOKWORM,
                ),
                "debian-11" | "bullseye" => (
                    format!(
                        "boinc-client_{}-{}_amd64_bullseye.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_AMD64_BULLSEYE,
                ),
                "debian-10" | "buster" => (
                    format!(
                        "boinc-client_{}-{}_amd64_buster.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_AMD64_BUSTER,
                ),
                "fedora-42" => (
                    format!(
                        "boinc-client-{}-{}.x86_64_fc42.rpm",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_X64_FC42_RPM,
                ),
                "fedora-41" => (
                    format!(
                        "boinc-client-{}-{}.x86_64_fc41.rpm",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_X64_FC41_RPM,
                ),
                "fedora-40" => (
                    format!(
                        "boinc-client-{}-{}.x86_64_fc40.rpm",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_X64_FC40_RPM,
                ),
                "opensuse-15.6" => (
                    format!(
                        "boinc-client-{}-{}.x86_64_suse15_6.rpm",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_X64_SUSE15_6_RPM,
                ),
                // Default to Ubuntu Noble for modern Linux
                _ => {
                    info!(
                        "Unknown distro '{}', defaulting to Ubuntu Noble package",
                        distro
                    );
                    (
                        format!(
                            "boinc-client_{}-{}_amd64_noble.deb",
                            BOINC_VERSION, BOINC_BUILD
                        ),
                        boinc_hashes::LINUX_AMD64_NOBLE,
                    )
                }
            };

            return Ok((format!("{}{}", base_url, filename), hash));
        }

        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        {
            let distro = self.detect_linux_distro();

            let (filename, hash) = match distro.as_str() {
                "ubuntu-24.04" | "noble" => (
                    format!(
                        "boinc-client_{}-{}_arm64_noble.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_ARM64_NOBLE,
                ),
                "ubuntu-22.04" | "jammy" => (
                    format!(
                        "boinc-client_{}-{}_arm64_jammy.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_ARM64_JAMMY,
                ),
                "ubuntu-20.04" | "focal" => (
                    format!(
                        "boinc-client_{}-{}_arm64_focal.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_ARM64_FOCAL,
                ),
                "debian-12" | "bookworm" => (
                    format!(
                        "boinc-client_{}-{}_arm64_bookworm.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_ARM64_BOOKWORM,
                ),
                "debian-11" | "bullseye" => (
                    format!(
                        "boinc-client_{}-{}_arm64_bullseye.deb",
                        BOINC_VERSION, BOINC_BUILD
                    ),
                    boinc_hashes::LINUX_ARM64_BULLSEYE,
                ),
                _ => {
                    info!(
                        "Unknown distro '{}', defaulting to Ubuntu Noble ARM64 package",
                        distro
                    );
                    (
                        format!(
                            "boinc-client_{}-{}_arm64_noble.deb",
                            BOINC_VERSION, BOINC_BUILD
                        ),
                        boinc_hashes::LINUX_ARM64_NOBLE,
                    )
                }
            };

            return Ok((format!("{}{}", base_url, filename), hash));
        }

        #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
        {
            let filename = format!("boinc_{}_windows_x86_64.exe", BOINC_VERSION);
            return Ok((
                format!("{}{}", base_url, filename),
                boinc_hashes::WINDOWS_X64_EXE,
            ));
        }

        #[cfg(all(target_arch = "aarch64", target_os = "windows"))]
        {
            let filename = format!("boinc_{}_windows_arm64.exe", BOINC_VERSION);
            return Ok((
                format!("{}{}", base_url, filename),
                boinc_hashes::WINDOWS_ARM64_EXE,
            ));
        }

        // Fallback for unsupported platforms
        #[allow(unreachable_code)]
        Err(anyhow::anyhow!(
            "No verified BOINC hash available for this platform. \
            Please set CHERT_BOINC_DOWNLOAD_URL and CHERT_BOINC_EXPECTED_HASH environment variables \
            with the SHA256 hash from https://github.com/BOINC/boinc/releases"
        ))
    }

    /// Detect Linux distribution from /etc/os-release
    fn detect_linux_distro(&self) -> String {
        // Try to read /etc/os-release
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            let mut id = String::new();
            let mut version_id = String::new();
            let mut version_codename = String::new();

            for line in content.lines() {
                if let Some(value) = line.strip_prefix("ID=") {
                    id = value.trim_matches('"').to_lowercase();
                } else if let Some(value) = line.strip_prefix("VERSION_ID=") {
                    version_id = value.trim_matches('"').to_string();
                } else if let Some(value) = line.strip_prefix("VERSION_CODENAME=") {
                    version_codename = value.trim_matches('"').to_lowercase();
                }
            }

            // Return codename if available (preferred for matching)
            if !version_codename.is_empty() {
                return version_codename;
            }

            // Otherwise return id-version
            if !id.is_empty() && !version_id.is_empty() {
                return format!("{}-{}", id, version_id);
            }

            if !id.is_empty() {
                return id;
            }
        }

        // Default fallback
        "unknown".to_string()
    }

    /// Download and install BOINC (or other archive/binary) to the install dir.
    /// Uses secure cryptographic verification with constant-time comparison.
    /// Supports: .tar.gz, .tgz, .zip, and single-file binaries.
    pub async fn download_and_install(
        &self,
        url: &str,
        expected_sha256: Option<String>,
    ) -> Result<()> {
        info!(dir = %self.install_dir.display(), url = %url, "download_and_install start");

        // SECURITY: Use secure HTTP client with certificate validation
        let client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout for large downloads
            .connect_timeout(std::time::Duration::from_secs(30))
            .user_agent("ChertMiner/1.0")
            .danger_accept_invalid_certs(false) // Always verify certificates
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()
            .context("Failed to create secure HTTP client for download")?;

        // SECURITY: Validate URL is HTTPS for binary downloads
        if !url.starts_with("https://") {
            return Err(anyhow::anyhow!(
                "SECURITY: Binary downloads must use HTTPS: {}",
                url
            ));
        }

        let resp = client.get(url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Download failed with status {}: {}",
                resp.status(),
                url
            ));
        }

        let bytes = resp.bytes().await?;

        // SECURITY: ALWAYS verify SHA256 for binary downloads using constant-time comparison
        match expected_sha256 {
            Some(expected) => {
                // Use shared crypto verification with constant-time comparison
                let verification_result = MinerHashUtils::verify_file_integrity(&expected, &bytes)
                    .context("Failed to verify file integrity")?;

                if !verification_result {
                    return Err(anyhow::anyhow!(
                        "SECURITY: SHA256 verification failed! \
                        Expected: {} \
                        Download may be compromised or hash is incorrect.",
                        expected
                    ));
                }

                info!("✅ SHA256 verification passed: {}", expected);
            }
            None => {
                // SECURITY: This should never happen in production
                return Err(anyhow::anyhow!(
                    "SECURITY: SHA256 verification is required for all binary downloads. \
                    Please provide expected hash via CHERT_BOINC_EXPECTED_HASH environment variable."
                ));
            }
        }

        let url_lc = url.to_ascii_lowercase();
        // Prepare install dir
        self.ensure_dirs()?;

        if url_lc.ends_with(".sh") {
            // SECURITY WARNING: Executing downloaded shell scripts is inherently risky
            warn!(
                "SECURITY: Downloading and executing shell script from {}",
                url
            );
            warn!("SECURITY: This should only be done with trusted sources and verified SHA256");

            // Additional validation - only allow scripts from trusted domains
            if !url.starts_with("https://github.com/BOINC/")
                && !url.starts_with("https://boinc.berkeley.edu/")
            {
                return Err(anyhow::anyhow!(
                    "SECURITY: Shell script downloads only allowed from trusted BOINC domains"
                ));
            }

            // Handle BOINC installer scripts
            let installer_path = self.install_dir.join("boinc_installer.sh");
            std::fs::write(&installer_path, &bytes)?;

            // Make executable and run installer
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&installer_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&installer_path, perms)?;
            }

            // Run installer (non-interactive)
            warn!("SECURITY: Executing installer with --noexec (extract only)");
            let output = tokio::process::Command::new(&installer_path)
                .arg("--noexec") // Extract but don't run - CRITICAL for security
                .current_dir(&self.install_dir)
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "BOINC installer failed: {:?}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            info!("BOINC installer extraction completed");
        } else if url_lc.ends_with(".deb") {
            // Handle Debian packages
            let deb_path = self.install_dir.join("boinc.deb");
            std::fs::write(&deb_path, &bytes)?;

            // Extract .deb using ar + tar
            let output = tokio::process::Command::new("ar")
                .arg("x")
                .arg(&deb_path)
                .current_dir(&self.install_dir)
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to extract .deb archive: {:?}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            // Extract data.tar.xz if it exists
            let data_tar = self.install_dir.join("data.tar.xz");
            if data_tar.exists() {
                let output = tokio::process::Command::new("tar")
                    .arg("xf")
                    .arg(&data_tar)
                    .current_dir(&self.install_dir)
                    .output()
                    .await?;

                if !output.status.success() {
                    return Err(anyhow::anyhow!(
                        "Failed to extract data.tar.xz: {:?}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
            }

            info!("Debian package extraction completed");
        } else if url_lc.ends_with(".tar.gz") || url_lc.ends_with(".tgz") {
            // Handle tar.gz archives
            let archive_path = self.install_dir.join("boinc.tar.gz");
            std::fs::write(&archive_path, &bytes)?;

            let output = tokio::process::Command::new("tar")
                .arg("xzf")
                .arg(&archive_path)
                .arg("-C")
                .arg(&self.install_dir)
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to extract tar.gz: {:?}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            info!("Tar.gz extraction completed");
        } else if url_lc.ends_with(".zip") {
            // Handle ZIP archives
            let zip_path = self.install_dir.join("boinc.zip");
            std::fs::write(&zip_path, &bytes)?;

            let output = tokio::process::Command::new("unzip")
                .arg("-o")
                .arg(&zip_path)
                .arg("-d")
                .arg(&self.install_dir)
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to extract zip: {:?}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            info!("ZIP extraction completed");
        } else {
            // Handle single binary downloads
            let binary_name = url.split('/').next_back().unwrap_or("boinc");
            let binary_path = self.install_dir.join(binary_name);
            std::fs::write(&binary_path, &bytes)?;

            // Make binary executable on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&binary_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&binary_path, perms)?;
            }

            info!("Binary download completed: {}", binary_path.display());
        }

        info!(bin = %self.binary_path.display(), "download_and_install complete");
        Ok(())
    }
}
