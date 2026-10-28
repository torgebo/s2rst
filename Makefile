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
#   * Gungraun       — deterministic instruction counts, used as the
#                      regression gate (formerly iai-callgrind; the
#                      `iai` names below are kept for continuity). Bench targets are the plain names
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

# Run the full Gungraun (callgrind) suite.
.PHONY: bench-iai
bench-iai: ## Run the Gungraun benchmarks (deterministic instruction counts; the regression gate)
	cargo bench -p s2rst $(foreach b,$(P0_IAI),--bench $(b))

# Run the full Criterion wall-clock suite.
.PHONY: bench-wall
bench-wall: ## Run the Criterion wall-clock benchmarks
	cargo bench -p s2rst $(foreach b,$(P0_WALL),--bench $(b))

# Both, in sequence.
.PHONY: bench-all
bench-all: bench-iai bench-wall ## Run both benchmark suites (Gungraun then Criterion)

# Native-CPU Criterion run — enables FMA/AVX2/AVX-512 if the host has them.
# Do NOT use for baselines; for tuning experiments only.
.PHONY: bench-native
bench-native: ## Run the wall-clock benches with target-cpu=native (FMA/AVX2/AVX-512); tuning only, not for baselines
	RUSTFLAGS="-C target-cpu=native" \
		cargo bench -p s2rst $(foreach b,$(P0_WALL),--bench $(b))

# Record baselines: capture both Gungraun and Criterion JSON output under
# $(BASELINE_DIR)/<date>-<rev>/.
#
# Exit gate for Phase 0: running this twice in a row must produce stable
# numbers (± Criterion noise bands).
.PHONY: baseline
baseline: ## Record Gungraun + Criterion baselines under core/benches/baselines/<date>-<rev>/
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

# ===== Formal verification (verify/, specs/001-formal-verify-base-type, 002-formal-verify-cell-union, 003-formal-verify-decoders, 004-formal-verify-cell-interval) =====
#
# Kani proofs for s2rst::s2::CellId (spec 001), s2rst::s2::CellUnion (spec 002), the
# decoding layer (spec 003) and the value types Cell and s1::Interval (spec 004), plus spec
# 003's deductive track: restated codecs proved with Verus in verify/deductive/. Not part of `make test`: they need the Kani and Verus
# toolchains. CI runs the same targets from .github/workflows/verify.yml. The CellUnion,
# decoding and value-type harnesses sit behind the verify crate's default `cellunion`,
# `decoding` and `values` features, so `--no-default-features` builds the CellId suite alone.
# Every run goes through verify/run-verify.sh, which caps each CBMC process's memory
# (VERIFY_MEM_PER_HARNESS, default 4G) and sizes VERIFY_JOBS from available memory.

KANI_VERSION := 0.68.0
# The Verus release of spec 003's deductive track (research R11), and its Rust toolchain.
VERUS_VERSION := 0.2026.09.20.aef82ed
VERUS_TOOLCHAIN := 1.98.1
VERUS ?= verus

.PHONY: verify-kani-check
verify-kani-check: ## Check that cargo-kani $(KANI_VERSION) is installed (used by the verify targets)
	@cargo kani --version 2>/dev/null | grep -q "Kani Rust Verifier $(KANI_VERSION)" || { \
		echo "verify: cargo-kani $(KANI_VERSION) is required. Install it with:"; \
		echo "  cargo install --locked kani-verifier --version $(KANI_VERSION) && cargo kani setup"; \
		exit 1; }

.PHONY: verify
verify: verify-kani-check ## Run the CellId, CellUnion and decoding proofs: model and link tests, then every Kani harness under the memory guard
	cd verify && cargo test
	cd verify/deductive && cargo test
	cd verify && ./run-verify.sh

.PHONY: verify-cellid
verify-cellid: verify-kani-check ## Run the CellId proofs only (spec 001)
	cd verify && cargo test
	cd verify && ./run-verify.sh --no-default-features

.PHONY: verify-union
verify-union: verify-kani-check ## Run the CellUnion proofs only (spec 002)
	cd verify && cargo test
	cd verify && ./run-verify.sh --harness cu_

# The decoding suite (spec 003), whole or one part (CI runs the parts as parallel legs, so
# each stays under SC-006's 10 minutes): P=vectors the fixed-width and integer vector
# properties, P=links the bounded agreement with the restated codecs and the string vector,
# P=codecs the cell-id and point vectors, the fixed-size codecs and the remaining crash
# proofs. The parts are balanced on harness time plus about 5 s of Kani set-up per harness.
DECODING_HARNESSES = $(if $(filter vectors,$(P)),--harness cd_fw:: --harness cd_uv::,$(if $(filter links,$(P)),--harness cd_agree:: --harness cd_sv::,$(if $(filter codecs,$(P)),--harness cd_cv:: --harness cd_pv:: --harness cd_fx:: --harness cd_crash::,--harness cd_)))

.PHONY: verify-decoding
verify-decoding: verify-kani-check ## Run the decoding-layer proofs only (spec 003) [P=<vectors|links|codecs>]
	cd verify && cargo test
	cd verify/deductive && cargo test
	cd verify && ./run-verify.sh $(DECODING_HARNESSES)

# The value types: P=cell runs the Cell harnesses (cl_*), P=interval the s1::Interval ones
# (iv_*) (spec 004); P=real the r1::Interval ones (ri_*), P=rect the s2::Rect ones (rl_*)
# (spec 005); P=uvrect the r2::Rect ones (ur_*), P=point the s2::Point ones (pt_*) (spec 006);
# P=chord the s1::ChordAngle ones (ca_*), P=angle the s1::Angle ones (ag_*) (spec 007); P=cap the
# s2::Cap ones (cp_*) (spec 008); P=latlng the s2::LatLng ones (ll_*) (spec 010; its ll_reg holds
# the rectangle's Region laws, which go through LatLng). Without P, all ten; CI runs each as a leg. Without P and with P=angle, the E-form round trips are also
# proven by enumerating every i32 in release mode (verify/tests/angle_enumeration.rs, about
# 70 s; spec 007 research R12). A part selects its modules by name
# (`--harness ri_exp::`): Kani matches a filter anywhere in a harness's name, so a bare prefix
# would also run other parts' harnesses (`ri_` matches `pt_ari::p_ptari_01`, `cl_` matches
# `pt_scl`, `iv_` matches `ri_val::p_riv_01`). P may instead list harness filters, each
# `<module>::<prefix>` (P='cl_bnd::p_cchd_ cl_bnd::p_ceq_'), as the weekly CI legs do.
values_part = $(foreach m,$(basename $(notdir $(wildcard verify/src/$(1)_*.rs))),--harness $(m)::)
VALUES_HARNESSES = $(if $(findstring ::,$(P)),$(addprefix --harness ,$(P)),$(if $(filter cell,$(P)),$(call values_part,cl),$(if $(filter interval,$(P)),$(call values_part,iv),$(if $(filter real,$(P)),$(call values_part,ri),$(if $(filter rect,$(P)),$(call values_part,rl),$(if $(filter uvrect,$(P)),$(call values_part,ur),$(if $(filter point,$(P)),$(call values_part,pt),$(if $(filter chord,$(P)),$(call values_part,ca),$(if $(filter angle,$(P)),$(call values_part,ag),$(if $(filter cap,$(P)),$(call values_part,cp),$(if $(filter latlng,$(P)),$(call values_part,ll),--harness cl_ --harness iv_ --harness ri_ --harness rl_ --harness ur_ --harness pt_ --harness ca_ --harness ag_ --harness cp_ $(call values_part,ll))))))))))))

.PHONY: verify-values
verify-values: verify-kani-check ## Run the value-type proofs only: Cell, s1::Interval (spec 004), r1::Interval, s2::Rect (spec 005), r2::Rect, s2::Point (spec 006), s1::ChordAngle, s1::Angle (spec 007), s2::Cap (spec 008), s2::LatLng (spec 010) [P=<cell|interval|real|rect|uvrect|point|chord|angle|cap|latlng|module::prefix ...>]
	cd verify && cargo test
	$(if $(filter-out angle,$(P)),,cd verify && cargo test --release --test angle_enumeration -- --ignored)
	cd verify && ./run-verify.sh $(VALUES_HARNESSES)

# The release pass (the documented panics, gap 1 of the 2026-10-26 coverage review): built
# without debug assertions, it runs the `n_*` harnesses whose panic is in every build (the
# debug-only ones compile out), the `r_*` harnesses, which prove that the debug checks a
# release build skips leave no panic or overflow on the same inputs, and the `o_*` ones (inputs
# outside a domain that return, in every build). Its own target directory
# (verify/target/no-debug-assertions), so it does not rebuild the debug one.
.PHONY: verify-release
verify-release: verify-kani-check ## Run the documented panics' release-build proofs (every-build n_*, r_*, o_*), without debug assertions
	cd verify && VERIFY_DEBUG_ASSERTIONS=0 ./run-verify.sh --harness ::n_ --harness ::r_ --harness ::o_

.PHONY: verify-verus-check
verify-verus-check: ## Check that the pinned Verus release and its Rust toolchain are installed [VERUS=<path to verus>]
	@$(VERUS) --version 2>/dev/null | grep -q "Version: $(VERUS_VERSION)" || { \
		echo "verify: Verus $(VERUS_VERSION) is required (found: $$($(VERUS) --version 2>/dev/null | awk '/Version:/ {print $$2}'))."; \
		echo "  Download the release from https://github.com/verus-lang/verus/releases"; \
		echo "  and run with VERUS=<unpacked dir>/verus."; \
		exit 1; }
	@rustup toolchain list | grep -q "^$(VERUS_TOOLCHAIN)-" || { \
		echo "verify: Verus needs Rust $(VERUS_TOOLCHAIN): rustup toolchain install $(VERUS_TOOLCHAIN)"; \
		exit 1; }

.PHONY: verify-deductive
verify-deductive: verify-verus-check ## Run the deductive track (spec 003): Verus on the restated codecs, then the link tests [MUTANTS=1]
	cd verify/deductive && VERUS=$(VERUS) VERUS_VERSION=$(VERUS_VERSION) MUTANTS=$(MUTANTS) ./run-verus.sh
	cd verify/deductive && cargo test

.PHONY: verify-deductive-bless
verify-deductive-bless: ## Rewrite the drift guard's fingerprints after reviewing the restated copies (spec 003)
	cd verify/deductive && BLESS=1 cargo test --test drift

.PHONY: verify-mutation-measure
verify-mutation-measure: ## Run the cargo-mutants measurement over the decoding layer, hours [PHASE=before|after]
	@test -n "$(PHASE)" || { echo "usage: make verify-mutation-measure PHASE=before|after"; exit 1; }
	cd verify && ./run-mutation-measure.sh $(PHASE)

.PHONY: verify-harness
verify-harness: verify-kani-check ## Run one proof harness under the memory guard [H=<module>::<fn>]
	@test -n "$(H)" || { echo "usage: make verify-harness H=<module>::<fn>"; exit 1; }
	cd verify && ./run-verify.sh --features slow --exact --harness $(H)

.PHONY: verify-mutants
verify-mutants: verify-kani-check ## Run the mutant (non-vacuity) campaign under the memory guard [M=<ids or patterns, e.g. 'M-5*'>]
	cd verify && ./run-mutants.sh "$(M)"

# The slow tier, whole or in parts: F=0…5 selects one face's neighbour proofs (the
# edge_neighbors crash proof and the P-NBR properties), F=vertex the vertex_neighbors crash
# proof (CI runs the parts as a matrix). F=union runs the whole CellUnion suite with its
# slow-tier members (spec 002): the larger property members and the 25-byte decode proofs,
# which need an 8G ceiling. F=decoding runs the whole decoding suite (spec 003) with its
# slow-tier members, the cell-id vector encoder at one and two ids, at an 8G ceiling; with
# P=vectors|links|codecs, only that part, as for verify-decoding (CI runs the three as legs).
# F=values runs the value-type suite (specs 004–006) with its slow-tier members, the deeper
# levels of the Cell bound families, any split interval member, the rectangle's expansion
# containment, the u-v rectangle's costlier members and the point's geometry, at an 8G
# ceiling; with P=cell|interval|real|rect|uvrect|point, only that part, and with P as a list
# of `<module>::<prefix>` filters, only those (CI's eight weekly values legs). Its 16 members
# that time out at 60 minutes are listed in verify/known-timeouts.txt and reported as
# KNOWN-TIMEOUT, which does not fail the run. The whole values tier takes about 24 hours one
# at a time (2026-10-14). Without F, every part runs except union, decoding and values.
#
# Every part runs one harness at a time with 60 minutes each, unless VERIFY_JOBS or
# VERIFY_HARNESS_TIMEOUT say otherwise (CI sets both: one job per physical core, the
# 30-minute budget). On the 4-core/8-thread workstation, the memory-sized default of 8
# jobs made 22 of the 24 P-NBR-03 proofs time out at 30 minutes, and 4 jobs took them to
# 29; run alone they take 12–18 minutes (2026-10-08, EVALUATION.md). The whole tier then
# takes about 13 hours; run-verify.sh prints each harness's result as it finishes.
SLOW_HARNESSES = $(if $(F),$(if $(filter vertex,$(F)),--exact --harness crash::c_vertex_neighbors,$(if $(filter union,$(F)),--harness cu_,$(if $(filter decoding,$(F)),$(DECODING_HARNESSES),$(if $(filter values,$(F)),$(VALUES_HARNESSES),--harness crash::c_edge_neighbors__f$(F) --harness nbr::p_nbr_01__f$(F)_ --harness nbr::p_nbr_03__f$(F)_)))),--harness nbr:: --harness crash::c_edge_neighbors --harness crash::c_vertex_neighbors)
SLOW_ENV = $(if $(filter union decoding values,$(F)),VERIFY_MEM_PER_HARNESS=$${VERIFY_MEM_PER_HARNESS:-8G})

.PHONY: verify-slow
verify-slow: verify-kani-check ## Run the weekly slow-tier proofs, one at a time, 60 min per harness [F=<0-5|vertex|union|decoding|values>] [P=<vectors|links|codecs> with F=decoding, <cell|interval|real|rect|uvrect|point|module::prefix ...> with F=values]
	cd verify && $(SLOW_ENV) VERIFY_JOBS=$${VERIFY_JOBS:-1} VERIFY_HARNESS_TIMEOUT=$${VERIFY_HARNESS_TIMEOUT:-60m} ./run-verify.sh --features slow $(SLOW_HARNESSES)

# The xlarge tier (spec 003): the decoding members over the weekly budget, for a separate
# machine with 32 GB of memory. One harness at a time at a 26G ceiling, 4 hours each; the
# cgroup backstop and the playback re-run stay below the available memory, as everywhere. The
# preflight refuses to start unless the ceiling plus 2 GiB is available, so it does not run
# on a smaller workstation by accident. No CI job runs it.
XLARGE_MEM ?= 26G
# Since the 2026-10-26 coverage review (verify/COVERAGE.md) the tier also holds five `Cell`
# float-geometry crash proofs that run out of memory at 8 GiB weekly.
XLARGE_VALUES = cl_crash::c_fg_exact_area cl_crash::c_fg_distance_to_edge cl_crash::c_fg_max_distance_to_edge cl_crash::c_fg_distance_to_cell cl_crash::c_fg_max_distance_to_cell
XLARGE_HARNESSES = $(if $(H),--exact --harness $(H),--harness cd_xlarge:: $(addprefix --harness ,$(XLARGE_VALUES)))

.PHONY: verify-xlarge
verify-xlarge: verify-kani-check ## Run the xlarge tier on a 32 GB machine: one harness at a time, 26G ceiling, 4 h each [H=<module>::<fn>] [XLARGE_MEM=<n>G]
	@case "$(XLARGE_MEM)" in *G) ;; *) echo "verify-xlarge: XLARGE_MEM must be in whole gigabytes, like 26G"; exit 1 ;; esac
	@avail=$$(awk '/^MemAvailable:/ {print int($$2 / 1048576)}' /proc/meminfo 2>/dev/null); \
	need=$$(( $(patsubst %G,%,$(XLARGE_MEM)) + 2 )); \
	if [ -z "$$avail" ] || [ "$$avail" -lt "$$need" ]; then \
		echo "verify-xlarge: needs $$need GiB of available memory for a $(XLARGE_MEM) ceiling; this machine has $${avail:-unknown} GiB."; \
		echo "  Run it on a machine with 32 GB, or lower XLARGE_MEM (the members may then run out of memory)."; \
		exit 1; fi
	cd verify && VERIFY_MEM_PER_HARNESS=$(XLARGE_MEM) VERIFY_JOBS=$${VERIFY_JOBS:-1} \
		VERIFY_HARNESS_TIMEOUT=$${VERIFY_HARNESS_TIMEOUT:-4h} VERIFY_RUN_DIR=$${VERIFY_RUN_DIR:-target/run-xlarge} \
		./run-verify.sh --features xlarge $(XLARGE_HARNESSES)
