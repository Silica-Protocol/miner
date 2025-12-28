# Chert Miner

**⛏️ Unified mining client for Chert blockchain with NUW (Network Utility Work) and BOINC scientific computing**

## Overview

The Chert miner provides a unified interface for earning rewards through:

1. **NUW (Network Utility Work)** - Solve useful challenges from the network (signature verification, ZK proof validation, Merkle proofs) in exchange for **fee discounts up to 50%**
2. **BOINC Scientific Computing** - Contribute to distributed science projects (Folding@Home, SETI@Home, etc.) for **mining rewards**

```
┌──────────────────────────────────────────────────────────────┐
│                      CHERT MINER                             │
├──────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐   │
│  │ NUW Worker  │    │   BOINC     │    │   Performance   │   │
│  │ (CPU)       │    │   Client    │    │   Monitor       │   │
│  │             │    │   (GPU)     │    │                 │   │
│  │ • Argon2    │    │             │    │ • CPU/GPU usage │   │
│  │ • Sig Batch │    │ • Projects  │    │ • Task progress │   │
│  │ • ZK Verify │    │ • Results   │    │ • Work stats    │   │
│  │ • Merkle    │    │ • Rewards   │    │                 │   │
│  └──────┬──────┘    └──────┬──────┘    └────────┬────────┘   │
│         │                  │                    │            │
│         └──────────────────┼────────────────────┘            │
│                            │                                 │
│                     ┌──────▼──────┐                          │
│                     │    Oracle   │                          │
│                     │   Server    │                          │
│                     └─────────────┘                          │
└──────────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Configure Environment

Copy the template and edit:
```bash
cp .env.template .env
# Edit .env with your settings
```

Required settings:
```env
CHERT_MINER_USER_ID=your_wallet_address
CHERT_ORACLE_URL=https://oracle.chert.network
```

### 2. Run the Miner

```bash
# Default mode (NUW on CPU + BOINC on GPU)
./target/release/miner

# NUW-only mode (no BOINC)
./target/release/miner --nuw-only

# BOINC-only mode (no NUW)
./target/release/miner --boinc-only

# TUI mode (interactive terminal UI)
./target/release/miner --tui

# Legacy mode (original BOINC-only behavior)
./target/release/miner --legacy
```

## Work Modes

| Mode | CPU Work | GPU Work | Use Case |
|------|----------|----------|----------|
| **Mixed** (default) | NUW | BOINC | Balanced earning |
| **NUW Only** | NUW | NUW | Fee discount focus |
| **BOINC Only** | BOINC | BOINC | Science computing focus |
| **GPU Only** | Idle | BOINC | Low CPU impact |

## NUW Challenge Types

| Type | Description | Fee Discount | Typical Time |
|------|-------------|--------------|--------------|
| `Argon2Pow` | Memory-hard PoW (fallback) | 0% | ~100ms |
| `SignatureBatch` | Verify transaction signatures | 25% | ~50-200ms |
| `ZkVerify` | Verify ZK proofs | 50% | ~1-5s |
| `MerkleVerify` | Validate state proofs | 30% | ~50ms |
| `PqAssist` | Post-quantum operations | 40% | ~200ms |

## Configuration Options

### Work Allocation

```env
# Enable/disable work types
CHERT_NUW_ON_CPU=true
CHERT_BOINC_ON_GPU=true

# Resource allocation (0-100)
CHERT_NUW_CPU_PERCENTAGE=25
CHERT_BOINC_GPU_PERCENTAGE=75

# Advanced settings
CHERT_NUW_ON_DEMAND=true
CHERT_MIN_NUW_DIFFICULTY=1000
CHERT_MAX_BOINC_TASKS=2
CHERT_AUTO_DETECT_HARDWARE=true
```

### Security

```env
CHERT_REQUIRE_HTTPS=true
CHERT_VERIFY_CERTIFICATES=true
CHERT_RATE_LIMIT_REQUESTS_PER_MINUTE=60
```

### Debug

```env
CHERT_DEBUG_MODE=false
CHERT_VERBOSE_LOGGING=false
```

## Hardware Detection

The miner automatically detects:
- CPU cores and architecture
- GPU vendor, model, and VRAM
- Available RAM
- Optimal work allocation

Example output:
```
Hardware: 8 cores, 1 GPU(s), 31.3 GB RAM
Recommended: NUW on CPU (25%), BOINC on GPU (75%)
```

## Building from Source

```bash
cd miner
cargo build --release
```

### Dependencies

- Rust 2024 edition
- OpenSSL (for HTTPS)
- BOINC client (optional, for BOINC work)

## Architecture

```
miner/
├── src/
│   ├── main.rs              # Entry point, CLI handling
│   ├── lib.rs               # Library exports
│   ├── miner_core.rs        # Main orchestrator
│   ├── nuw_worker.rs        # NUW challenge solver
│   ├── boinc/               # BOINC client integration
│   │   ├── mod.rs
│   │   ├── process_management.rs
│   │   └── configuration.rs
│   ├── config.rs            # Configuration management
│   ├── hardware_detection.rs # Hardware capabilities
│   ├── performance_monitor.rs # Metrics collection
│   └── oracle_profile.rs    # Oracle communication
└── Cargo.toml
```

## API Reference

### MinerCore

```rust
use miner::{MinerCore, MinerConfig, WorkMode, run_miner};

// Create and run miner
let config = MinerConfig::from_env()?;
run_miner(config).await?;

// Or manually control
let mut miner = MinerCore::new(config);
miner.initialize().await?;
miner.set_work_mode(WorkMode::NuwOnly);
miner.start().await?;
```

### NUW Worker

```rust
use miner::{NuwWorker, NuwChallenge, NuwSolution};

let worker = NuwWorker::new(&config);
worker.start().await?;  // Runs in background

// Check stats
let stats = worker.stats();
println!("Solved: {}", stats.challenges_solved.load(Ordering::Relaxed));
```

## Troubleshooting

### "BOINC not found"

Install BOINC client:
```bash
# Ubuntu/Debian
sudo apt install boinc-client

# Or use auto-install (coming soon)
./target/release/miner --install-boinc
```

### "Oracle connection failed"

- Check `CHERT_ORACLE_URL` is correct
- Verify HTTPS is available (or set `CHERT_REQUIRE_HTTPS=false` for testing)
- The miner will continue offline and retry

### "Configuration error"

Required environment variables:
- `CHERT_MINER_USER_ID` - Your wallet address
- `CHERT_ORACLE_URL` - Oracle server URL

## License

MIT License - See LICENSE file

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development guidelines.
