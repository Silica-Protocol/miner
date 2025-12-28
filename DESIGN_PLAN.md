## Mining — DESIGN_PLAN (living)

Updated: 2025-09-21

Goal
------
Define and track a living development plan for the mining subsystem that documents a client-driven architecture (avoid FFI where possible), necessary migrations, security and PQ requirements, tests, and a prioritized roadmap of concrete tasks mapped to files in this repo.

High-level summary
-------------------
- Move from C/FFI-based integrations (libboinc / FAH FFI) to a client-driven architecture where the miner spawns or controls standalone clients (BOINC/FAH) through safe protocols (XML-RPC/HTTP/CLI) or a lightweight bundled helper, keeping Rust code safe (no unsafe/FFI) except in a thin, isolated shim.
- Implement PoUW batching and Merkle receipts, mini-DAG back-refs, DAS overlay, QC compression, and Dilithium PQ signatures across receipts/QCs.
- Enforce secure transport (TLS 1.3 + rustls), certificate pinning where appropriate, and run `cargo audit` and `clippy` as part of the pipeline.

Living checklist (requirements coverage)
--------------------------------------
1) Client-driven BOINC/FAH integration (replace bulk FFI) — Status: ✅ **POI COMPLETED** / 🔄 **MINER IN PROGRESS**
2) PoUW multi-job receipts with Merkle root — Status: ✅ **COMPLETED** (Sept 21)
3) Mini-DAG back-references in worker batches — Status: planned
4) DAS mempool overlay (erasure coding + sampling) — Status: planned
5) QC compression (bitmap + concatenated Dilithium sigs) — Status: planned
6) Headless miner automation & account-based config — Status: partial (docs present)
7) Post-quantum signatures (Dilithium) across miner outputs — Status: ✅ **FOUNDATION COMPLETED** (Sept 21)
8) Secure transport (reqwest + rustls + cert pinning) — Status: ✅ **COMPLETED** (Sept 21)
9) Tests, CI, cargo-audit, clippy enforcement — Status: 🔄 **IN PROGRESS**
10) Documentation & explorer-facing telemetry for miners — Status: partial (docs exist in boinc.optimisations.md)

Where we are now (review of repository state)
---------------------------------------------
- FFI present: `src/ffi.rs`, `src/fah_ffi.rs`, local BOINC bindings in `BOINC/lib/boinc_api_bindings.rs` and `libboinc_api.so` under `BOINC/lib`. These are workable but expose unsafety, linking complexity and platform friction.
- PoUW-related artifacts: `src/oracle.rs`, `pouw_*` files in `poi/` (proof logic split between miner and poi). `PQ_MIGRATION_PLAN.md` exists in both `miner/` and `poi/`.
- BOINC automation & docs: `boinc.optimisations.md`, `BOINC/` folder with headers / libs and build scripts.

Design decisions and rationale
------------------------------
- Prefer running BOINC/FAH as a userland process controlled by the miner (spawn, supervise, configure), communicating over well-documented RPC (XML-RPC) or CLI flags. This avoids long-term coupling to C-APIs and eliminates large binary .so shipping issues.
- Keep a very small, audited FFI shim only when impossible (e.g., tightly coupled native-only library). Where an FFI shim remains, make it an optional Cargo feature behind `ffi` and keep tests and review focused there.
- Implement PQ crypto (Dilithium) at the application layer for receipts and QCs. Use a maintained Rust crate or a small, audited binding. Centralize signing and verification into `crypto/` utilities.
- Use TLS via `reqwest`+`rustls` for all network calls; add an optional certificate-pin store and acceptable root list controlled by config.

Migration tasks (FFI → client-driven)
-----------------------------------
Priority A — unblock developer friction and platform portability

1. Create `src/boinc_client.rs` (new): 🔄 **CURRENT FOCUS (Sept 21-30)**
   - Purpose: a pure-Rust client to communicate with a BOINC client instance over XML-RPC / HTTP or control a local BOINC binary via CLI.
   - Dependencies: ✅ POI Oracle provides reference implementation and crypto foundation
   - Status: 🔄 Adapt POI's `boinc_client.rs` for miner-side operations
   - Integration: Use POI's `CryptoEngine` and `WorkReceipt` structures for consistency

2. Add `src/boinc_automation.rs` (new): ⏭️ **NEXT PHASE**
   - Purpose: automates installation/checking of a BOINC binary (download, verify checksum, unpack), manages `boinc --daemon` lifecycle, and generates `cc_config.xml` per miner.
   - Foundation: ✅ POI's project management provides patterns for BOINC lifecycle management

3. Deprecate big FFI surface (iterative): 🔄 **ACTIVE**
   - Short term: ✅ POI demonstrates compatibility layer approach with `boinc_compat.rs`
   - Medium term: 🔄 Convert miner call sites to use client API (leverage POI patterns)
   - Long term: ⏭️ Remove `ffi.rs` and `BOINC/lib` from repo

4. FAH (Folding@home) path: ⏭️ **PLANNED**
   - Add `src/fah_client.rs` parallel to the BOINC client idea
   - Leverage: ✅ POI's XML processing and HTTP client patterns

Priority B — correctness, security, and PQ ✅ **FOUNDATION AVAILABLE**

5. Implement `crypto/dilithium.rs` (or `src/crypto.rs`): ✅ **FOUNDATION COMPLETED**
   - Status: ✅ POI Oracle provides complete cryptographic engine with Ed25519 and Dilithium upgrade path
   - Integration: 🔄 Import and adapt POI's `crypto.rs` for miner operations
   - Use: Apply in `src/oracle.rs`, `src/randomx_miner.rs`, and integrate with POI receipt verification

6. Add `src/receipt.rs` and change PoUW receipt format: ✅ **COMPLETED**
   - Status: ✅ POI Oracle implements `WorkReceipt` with Merkle tree integration
   - Format: Multi-job receipt with `Vec<JobDigest>`, Merkle root, and cryptographic signatures
   - Integration: 🔄 Miner generates receipts compatible with POI verification pipeline

Priority C — performance & data availability features

7. Add `src/das.rs`: erasure encode batches and expose chunking + sampling APIs. Use `reed-solomon-erasure` crate.
8. Add `src/batch.rs` updates: batch header to include `parents: Vec<Hash>` (k=3 by default) and `pre_final_bitmap` tracking.
9. Add `src/qc.rs` for QC compression (bitmap + concatenated signatures) and parallel verification helpers.

File-level migration mapping (concrete)
-------------------------------------
- `src/ffi.rs` -> replace by `src/boinc_client.rs` + `src/boinc_automation.rs` then remove.
- `src/fah_ffi.rs` -> replace by `src/fah_client.rs` then remove.
- `src/oracle.rs` -> refactor to use `src/receipt.rs`, `src/crypto.rs`, `src/qc.rs` and `boinc_client`.
- `src/randomx_miner.rs` -> add `parents` to Batch; use `src/das.rs` for encoding and `src/crypto.rs` for signing.
- `BOINC/` build artifacts -> keep only reference build scripts for packaging; not required at runtime once client-driven approach works.

Security, PQ and transport tasks
--------------------------------
- Enforce TLS for all outbound RPC: require `reqwest` + `rustls`. Add config options for cert pinning.
- Centralize key/cert handling in `src/crypto.rs`.
- Apply Dilithium signatures to:
  - PoUW receipts and Merkle roots
  - QCs and pre-final markers
  - DA attestations for chunk sampling
- Add rate-limiting and input validation in all public RPC handlers (main.rs / API surface).

Testing, CI and validation
-------------------------
- Add integration tests that spawn a minimal local BOINC/FAH binary or a mocked XML-RPC server to validate `boinc_client` behavior.
- Add unit tests for Merkle receipts, Dilithium signatures, QC compression.
- Add CI tasks:
  - `cargo check` and `cargo clippy`
  - `cargo test` with fixtures
  - `cargo audit`

Developer ergonomics & ops
--------------------------
- Keep an optional `--with-ffi` feature to allow building with legacy libs during migration but default to client-driven implementation.
- Provide `scripts/dev_boot.sh` that brings up a test BOINC XML-RPC mock and runs the miner integration tests.
- Provide `docs/boinc-dev.md` and `docs/fah-dev.md` with developer instructions and recorded RPC fixtures.

Priority roadmap (quarterly cadence, living)
-------------------------------------------
Q1 (0–8 weeks): 🔄 **CURRENT FOCUS** (Sept 21, 2025)
- ✅ **DEPENDENCY COMPLETED**: POI Oracle client-driven architecture provides foundation
- 🔄 Implement miner-side `src/boinc_client.rs` + `src/boinc_automation.rs` with XML-RPC fixtures
- 🔄 Add compatibility shim `src/boinc_compat.rs` and wire into `main.rs` and `oracle.rs`
- ⏭️ Add unit tests and `scripts/dev_boot.sh`

Q2 (8–16 weeks): ⏭️ **UPCOMING**
- ⏭️ Implement `src/receipt.rs` and `src/crypto.rs` (integrate with POI crypto foundation)
- ⏭️ Add `src/batch.rs` changes for parents/back-refs and tests
- ⏭️ Run `cargo audit` and fix dependency issues

Q3 (16–28 weeks): ⏭️ **FUTURE**
- ⏭️ Implement `src/das.rs`, `src/qc.rs`, and integrate sampling attestations
- ⏭️ Replace legacy FFI files incrementally; remove `ffi` feature and `BOINC/lib` binary artifacts

Q4 (28–40 weeks): ⏭️ **PRODUCTION**
- ⏭️ Performance tuning, QC verification parallelism, DAS scaling tests
- ⏭️ Production hardening (TLS pins, cert rotation docs)
- ⏭️ Final removal of FFI and package cleanup

Acceptance criteria
-------------------
- `boinc_client` passes unit tests with recorded XML-RPC fixtures.
- PoUW receipts are signed with Dilithium and verified by `poi` (integration test).
- `cargo audit` returns no high/critical advisories (or known, tracked exceptions).
- CI pipeline runs `cargo clippy` and `cargo test` on every PR.

Open risks & mitigations
------------------------
- Risk: Some BOINC integrations or platform-specific features only available via native API.
  - Mitigation: Keep a small, well-audited optional FFI shim behind a feature flag; seek to upstream XML-RPC endpoints or maintain a tiny compatibility process.
- Risk: Dilithium crate maturity.
  - Mitigation: Evaluate multiple PQ implementations, prefer well-maintained crates or a minimal vetted binding; keep abstraction layer to swap impls.
- Risk: Developer friction while converting existing deployments.
  - Mitigation: Provide a compatibility mode and migration guide; keep backward compatible behavior until cutover.

Notes & references
------------------
- See `PQ_MIGRATION_PLAN.md` in this folder for more background on Dilithium adoption.
- See `miner/boinc.optimisations.md` for headless/miner UX and automation notes.

How to contribute & maintain this plan
-------------------------------------
- Edit this file with concrete deltas as tasks are completed.
- When adding or removing modules, add a short changelog entry at the top with date and author.
- Run `cargo audit` and update `SECURITY.md` on any dependency exceptions.

Quick-start developer commands
------------------------------
Run tests and checks locally:
```fish
# run type checks and linters
cargo clippy 2>&1 | tee miner-clippy.log

# run unit tests
cargo test 2>&1 | tee miner-test.log

# run security audit
cargo audit 2>&1 | tee miner-audit.log
```

Completion summary
------------------
This document creates a living roadmap for the mining subsystem, replacing brittle FFI with a client-driven approach, adding PQ signatures, improving DA and receipts, and enforcing secure transport. It maps concrete file-level changes, a staged migration, tests, and acceptance criteria.

---
Edit history
 - 2025-09-21: **CROSS-COMPONENT UPDATE** — POI Oracle foundation completed, miner integration updated
   - ✅ POI client-driven architecture provides crypto and Merkle foundation for miner integration
   - 🔄 Updated miner roadmap to leverage completed POI infrastructure
   - 📊 **Integration Status**: POI→Miner crypto pipeline established, receipt verification ready
 - 2025-09-10: Initial draft — created by automated plan generator
