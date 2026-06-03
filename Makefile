.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help (the default target)
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z][a-zA-Z0-9_-]*:.*## / {printf "  \033[36m%-13s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: check
check: ## Type-check the whole workspace (all features)
	cargo check --workspace --all-features

.PHONY: fmt-check
fmt-check: ## Check Rust formatting without modifying files (cargo fmt --check)
	cargo fmt --check

.PHONY: clippy
clippy: ## Lint the whole workspace, warnings as errors
	cargo clippy --workspace --all-features -- -D warnings

.PHONY: test-core
test-core: ## Run the core (s2rst) Rust test suite
	cargo test -p s2rst --all-features

.PHONY: doc
doc: ## Build the s2rst API docs, warnings as errors
	RUSTDOCFLAGS="-D warnings" cargo doc -p s2rst --all-features --no-deps

.PHONY: test
test: ## Run the full local verification suite (check, fmt, clippy, core/python/wasm tests, docs, cargo-deny)
	$(MAKE) check
	$(MAKE) fmt-check
	$(MAKE) clippy
	$(MAKE) test-core
	$(MAKE) doc
	$(MAKE) -C python test
	$(MAKE) -C wasm test
	$(MAKE) deny

# ===== Phase 0 benchmarking infrastructure =====
#
# Two harnesses live side-by-side:
#   * iai-callgrind  — deterministic instruction counts, used as the
#                      regression gate. Bench targets are the plain names
#                      (predicates, point_angle, edge_crossing, ...).
#   * Criterion       — wall-clock, used to detect ILP/FMA/SIMD wins that do
#                       not show in instruction count. Bench targets are
#                       prefixed with wall_ (wall_predicates, ...).
#
# Always drive via `--bench <name>`; a bare `cargo bench` would run every
# target in the crate (~50 of them).

# P0 bench set.
P0_IAI  := predicates point_angle edge_crossing
P0_WALL := wall_predicates wall_point_angle wall_edge_crossing

BASELINE_DIR := core/benches/baselines

.PHONY: deny
deny: ## Audit dependencies for license/security/advisory issues (cargo-deny)
	cargo-deny check

# Run the full iai-callgrind suite.
.PHONY: bench-iai
bench-iai: ## Run the iai-callgrind benchmarks (deterministic instruction counts; the regression gate)
	cargo bench -p s2rst $(foreach b,$(P0_IAI),--bench $(b))

# Run the full Criterion wall-clock suite.
.PHONY: bench-wall
bench-wall: ## Run the Criterion wall-clock benchmarks
	cargo bench -p s2rst $(foreach b,$(P0_WALL),--bench $(b))

# Both, in sequence.
.PHONY: bench-all
bench-all: bench-iai bench-wall ## Run both benchmark suites (iai-callgrind then Criterion)

# Native-CPU Criterion run — enables FMA/AVX2/AVX-512 if the host has them.
# Do NOT use for baselines; for tuning experiments only.
.PHONY: bench-native
bench-native: ## Run the wall-clock benches with target-cpu=native (FMA/AVX2/AVX-512); tuning only, not for baselines
	RUSTFLAGS="-C target-cpu=native" \
		cargo bench -p s2rst $(foreach b,$(P0_WALL),--bench $(b))

# Record baselines: capture both iai and Criterion JSON output under
# $(BASELINE_DIR)/<date>-<rev>/.
#
# Exit gate for Phase 0: running this twice in a row must produce stable
# numbers (± Criterion noise bands).
.PHONY: baseline
baseline: ## Record iai + Criterion baselines under core/benches/baselines/<date>-<rev>/
	@set -e; \
	rev=$$(git rev-parse --short HEAD); \
	date=$$(date -u +%Y%m%d); \
	out=$(BASELINE_DIR)/$$date-$$rev; \
	mkdir -p $$out; \
	echo "Writing baseline to $$out"; \
	rustc --version > $$out/rustc.txt; \
	uname -a > $$out/host.txt; \
	for b in $(P0_IAI); do \
		cargo bench -p s2rst --bench $$b 2>&1 | tee $$out/iai_$$b.txt; \
	done; \
	for b in $(P0_WALL); do \
		cargo bench -p s2rst --bench $$b -- --save-baseline main 2>&1 | tee $$out/$$b.txt; \
	done

# ===== Measurement tooling =====
# Usage: make perf-record BENCH=wall_predicates FILTER=sign
BENCH  ?= wall_predicates
FILTER ?=

.PHONY: perf-record
perf-record: ## Record a perf profile of a bench [BENCH=<name> FILTER=<filter>]
	@which perf >/dev/null || { echo "perf not installed"; exit 1; }
	cargo bench -p s2rst --bench $(BENCH) --no-run
	bin=$$(ls -t target/release/deps/$(BENCH)-* | grep -v '\.d$$' | head -1); \
	perf record -F 2000 --call-graph dwarf -o perf.data -- $$bin --bench --profile-time 5 $(FILTER)
	@echo "Run: perf report -i perf.data"

.PHONY: perf-report
perf-report: ## Open the last recorded perf profile (perf.data)
	perf report -i perf.data

.PHONY: cachegrind
cachegrind: ## Profile a bench with Valgrind cachegrind [BENCH=<name> FILTER=<filter>]
	@which valgrind >/dev/null || { echo "valgrind not installed"; exit 1; }
	cargo bench -p s2rst --bench $(BENCH) --no-run
	bin=$$(ls -t target/release/deps/$(BENCH)-* | grep -v '\.d$$' | head -1); \
	valgrind --tool=cachegrind --cachegrind-out-file=cachegrind.out $$bin --bench --profile-time 1 $(FILTER)
	cg_annotate cachegrind.out | head -200

.PHONY: flamegraph
flamegraph: ## Generate a flamegraph for a bench [BENCH=<name> FILTER=<filter>]
	@which cargo-flamegraph >/dev/null || { echo "cargo install flamegraph"; exit 1; }
	cargo flamegraph --bench $(BENCH) -- --bench --profile-time 5 $(FILTER)

# Dump optimized assembly of a function to inspect FMA/SIMD emission.
# Usage: make asm FUNC=s2rst::s2::predicates::sign
FUNC ?= s2rst::r3::vector::Vector::dot
.PHONY: asm
asm: ## Dump a function's optimized assembly to inspect FMA/SIMD emission [FUNC=<path>]
	@which cargo-show-asm >/dev/null || { echo "cargo install cargo-show-asm"; exit 1; }
	cargo asm --release --lib "$(FUNC)"

# ===== Phase 8: Profile-Guided Optimization =====
#
# `make pgo` rebuilds the P0 wall benches in the three-step PGO workflow
# (book §"Optional PGO support"):
#   1. instrument: `-Cprofile-generate=/tmp/s2rst-pgo-data` rebuild.
#   2. profile:    run the P0 wall benches; instrumentation drops .profraw
#                  files into the profile-generate dir.
#   3. optimize:   `llvm-profdata merge` then a third rebuild with
#                  `-Cprofile-use` and a final bench run for comparison.
#
# Total runtime is ~15 minutes (three full bench-profile builds). Profile
# data is host- and workload-specific, so we ship the workflow, not the
# .profdata file.
LLVM_PROFDATA := $(shell rustc --print sysroot)/lib/rustlib/$(shell rustc -vV | sed -n 's|host: ||p')/bin/llvm-profdata
PGO_DATA_DIR := /tmp/s2rst-pgo-data
PGO_PROFDATA := /tmp/s2rst.profdata

.PHONY: pgo
pgo: ## Run the 3-step Profile-Guided Optimization workflow on the wall benches (~15 min)
	rm -rf $(PGO_DATA_DIR)
	mkdir -p $(PGO_DATA_DIR)
	@echo "[1/3] Building instrumented benches…"
	RUSTFLAGS="-Cprofile-generate=$(PGO_DATA_DIR)" \
		cargo bench -p s2rst $(foreach b,$(P0_WALL),--bench $(b)) -- --save-baseline pre-pgo
	@echo "[2/3] Merging profile data…"
	$(LLVM_PROFDATA) merge -o $(PGO_PROFDATA) $(PGO_DATA_DIR)/*.profraw
	@echo "[3/3] Rebuilding with profile-use and comparing…"
	RUSTFLAGS="-Cprofile-use=$(PGO_PROFDATA)" \
		cargo bench -p s2rst $(foreach b,$(P0_WALL),--bench $(b)) -- --baseline pre-pgo
