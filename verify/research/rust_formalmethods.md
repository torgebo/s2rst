# Machine-Checked Proofs from Rust's Type System: Solvers, Verifiers and Proof Assistants such as Lean

## In brief
- Three workable approaches exist, and each leans on ownership and borrowing to keep proofs manageable: **translation into a proof assistant** (Aeneas for Lean 4, Coq, F* or HOL4; hax; rocq-of-rust), **verifiers inside Rust that call SMT solvers** (Verus, Creusot, Prusti, Kani, Flux), and **encodings in Rust's own types** (typestate, branded or ghost tokens, trait resolution as a logic engine).
- By September 2026 Lean 4 is a practical target: Aeneas's Lean backend carried Microsoft's SymCrypt work on SHA-3 and ML-KEM, and hax 0.4.0 routes Lean output through Charon and Aeneas. Ownership is the enabler, because it lets Aeneas build a pure functional model free of heap and aliasing reasoning.
- Choose by need: type-level encodings for cheap invariants checked on every build, an SMT verifier (Verus, Creusot) for automated functional correctness with a moderate trusted base, and Lean or Coq for kernel-checked proofs or mathematically rich specifications.

## Key findings
- **Aeneas** (Inria) maps MIR through its Low-Level Borrow Calculus (LLBC) to a pure lambda calculus, covering only safe code without interior mutability.
- **Charon**, shared by Aeneas and hax, lowers MIR to an unstructured graph (ULLBC) and then structured LLBC, exported as JSON.
- **hax** (Cryspen, successor to hacspec) targets F*, Rocq/Coq, ProVerif, SSProve, EasyCrypt and Lean; since 0.4.0 its Lean path runs through Charon and Aeneas, and the old backend is now "legacy-lean".
- **rocq-of-rust** (Formal Land) embeds Rust from THIR into Rocq/Coq and has been applied to revm, a Rust Ethereum VM.
- **SMT verifiers** avoid separation-logic heap reasoning through ownership but differ sharply in automation and trust.
- **Type-level techniques** turn the compiler into a modest proof checker; trait resolution amounts to clause solving (Chalk).
- Rust also hosts proof tooling: a Lean 4 kernel (nanoda_lib), SMT bindings and e-graph libraries.

## Details

### 1. Translating Rust into a proof assistant

**Aeneas and Charon (Inria).** The founding paper (Ho and Protzenko, "Aeneas: Rust verification by functional translation", OOPSLA 2022, DOI 10.1145/3547647) gives LLBC a value-based semantics with no addresses or pointer arithmetic and checks borrows by a semantic loan criterion instead of syntactic lifetimes; unsafe code and interior mutability are deferred. Charon reads MIR, builds the analysis-friendly ULLBC, restores structured control flow as LLBC and serializes it (both projects live under AeneasVerif). In May 2025 the Lean backend gained lemmas and tactics and moved to Lean v4.19.0. The workflow is Charon to produce an `.llbc` file, then Aeneas with `-backend lean`.

**Lean 4 in 2026.** Microsoft's SymCrypt paper (arXiv:2609.15648, 14 Sep 2026; Ho, Fournet, Protzenko, Naehrig and others) verified production Rust ports of SHA-3 and ML-KEM. The authors credit ownership for an extraction free of pointer-lifetime and aliasing reasoning, and AI agents write the Lean proofs, which the kernel checks independently. Microsoft Research confirms this code ships in the cryptographic provider behind Windows and Azure. An Ethereum Foundation zkEVM report (arXiv:2605.30106) lifts Plonky3 and RISC Zero cryptography into Lean 4 with Mathlib-grade specifications through both Aeneas and hax, using the AI provers Aristotle and Aleph under kernel checking, and notes friction from Lean toolchain drift and from extraction limits.

**hax (Cryspen).** A rustc frontend built with Inria, and shared with Charon, exports enriched ASTs that an OCaml engine rewrites through typed phases. The 0.4.0 Lean backend needs only cargo-hax and Charon. Annotations come from `hax-lib` (`requires`, `ensures`, `prop`/`int`). The rewrite from hacspec, a functional subset for executable crypto specifications, began in December 2023.

**rocq-of-rust (Formal Land).** The embedding is verbose because every borrow, dereference and effect is explicit. Its pipeline translates to `.v` files, links trait and function lookups to concrete definitions through `Run.Trait` instances, and simulates the result as a functional model. The Ethereum Foundation and Aleph Zero Foundation fund it.

**RustHorn and RustHornBelt.** RustHorn (Matsushita, Tsukada, Kobayashi; ESOP 2020, TOPLAS DOI 10.1145/3462205) reduces pointer-heavy Rust to first-order constrained Horn clauses, modelling a mutable borrow as a prophecy variable that predicts its final value, so no separation logic is needed. RustHornBelt (PLDI 2022) proves this sound against RustBelt/Iris. Creusot and RefinedRust later adopted prophecies.

### 2. Verifiers backed by SMT solvers

**Verus** (Lattuada et al., OOPSLA 2023, DOI 10.1145/3586037) generates weakest-precondition conditions for Z3. It separates `spec`, `proof` and `exec` code and never models the heap, because `&mut` arguments cannot alias; tracked ghost permissions handle raw pointers, interior mutability and concurrency. Microsoft and Amazon use it, two of three OSDI 2024 best papers built on it (also SOSP 2024, DOI 10.1145/3694715.3695952), and showcases include the verified-nrkernel page table (288 verified, 0 errors) and the VeriSMo confidential-VM security module.

**Creusot** (Denis, Jourdan, Marché; ICFEM 2022) compiles Rust to Coma and lets Why3 dispatch conditions to CVC5 or Z3. A `&mut T` is a current value plus a prophesied final one, and specifications are first-order logic. Trusting rustc's borrow checker makes it much faster than Prusti, which takes about ten times longer on simple examples.

**Prusti** (ETH Zurich) runs on the Viper separation-logic stack and, by default, proves from types alone that overflow and panics cannot occur; it introduced pledges for reborrows. It checks ownership inside Viper, which is heavier and slower than Creusot's approach.

**Kani** (AWS; arXiv:2607.01504) compiles MIR harnesses into CBMC, a bounded model checker that produces a SAT or SMT query. Without annotations it checks memory safety, undefined behaviour and panics, targeting what types cannot guarantee; contracts, quantifiers and stubbing extend it past bounded proofs. It verifies s2n-quic and AWS protobuf code and leads the Verify Rust Std Lib effort. Releases are in the 0.60–0.66 range, with cvc5, bitwuzla and z3 available as solvers.

**Flux** (Lehmann, Geller, Vazou, Jhala; PLDI 2023, DOI 10.1145/3591283) adds liquid refinement types as a rustc plugin, grounding its aliasing assumptions in Stacked Borrows. Its distinctive strength is inferring refinements, including quantified loop invariants that other tools make you write.

**RefinedRust** (Gäher et al., PLDI 2024, DOI 10.1145/3656422) is a foundational refinement type system proved sound in Coq against a RustBelt-based Iris model. It handles unsafe as well as safe code, its trusted base excludes the borrow checker, and IBM Research's ACE security monitor is an ongoing target.

**Automation against trust.** The SMT tools automate heavily but rely on the solver, their encoding and, for Creusot, Verus and Prusti, the borrow checker. Extraction and foundational systems give kernel-checked guarantees with a smaller trusted base, at the cost of more manual proof, which AI provers increasingly absorb.

### 3. Types as a proof mechanism

**Traits as logic (Chalk).** Generic trait solving needs first-order hereditary Harrop clauses, which allow `forall` and implication in clause bodies. Chalk cast it as a queryable Prolog-style program; it has given way to rustc's next-generation solver, but the logic view remains (arXiv:2309.05137).

**GhostCell** (Yanovski, Dang, Jung, Dreyer; ICFP 2021) splits data from a zero-sized token, linked by a brand lifetime that serves as a unique name, so one token governs many cells and safe, zero-cost aliased mutation becomes possible, with soundness proved in Iris. Crates include `ghost-cell`, `qcell` and `static-rc`; the cost is working inside a closure and threading the token everywhere.

**Typestate and related patterns.** Typestate gives each state its own type and consumes the old value on each transition, so misuse fails to compile; sealed traits block outside implementations. Helper crates include `static_assertions` (1.1.0), `typenum` (1.20.1, about 74 million monthly downloads, 75,292 dependent crates) and `nutype` (validated newtypes). Const generics are stable since 1.51, but arithmetic on them still needs the unstable `generic_const_exprs`. With neither dependent nor higher-kinded types, arithmetic, quantifiers and value-dependent invariants need an external solver.

### 4. Proof tools written in Rust

**nanoda_lib** is a small, auditable Lean 4 kernel that re-checks exported proofs, shrinking Lean's trusted base; its nanoclo fork speeds up large developments.

**Solvers and e-graphs.** `z3` (0.20.x) wraps Z3 and is maintained; `rsmt2` (0.16.2) drives any SMT-LIB 2 solver but has been idle for over three years; `easy-smt` runs a solver subprocess. `egg` (POPL 2021 Distinguished Paper, DOI 10.1145/3434304) provides e-graphs and `egglog` (PLDI 2023, DOI 10.1145/3591239) combines them with Datalog.

**Generating code and proofs.** Aeneas emits stubs for external definitions and hax writes monadic Lean. In Rust, traits with PhantomData can make malformed proof terms unrepresentable, and syn/quote macros can emit code and specifications together.

### 5. Proof obligations from the compiler

**Contracts.** Compiler MCP-759 proposes `#[contracts::requires]`, `#[contracts::ensures]` and type invariants, usable at run time or by verifiers; about 200 functions in the verify-rust-std fork already carry contracts.

**MIR as common ground.** Charon, Kani, Prusti, Creusot and Aeneas read MIR (rocq-of-rust reads THIR), and StableMIR gives them a stable interface.

**The standard-library challenge.** AWS and the Rust Foundation run a tool-neutral effort to verify memory safety and absence of undefined behaviour in std, which has roughly 35K functions, 7.5K of them unsafe. AWS cites 57 soundness issues and 20 CVEs in three years, 28% of them from 2024. Awards reach 10,000 USD per challenge; of the first nine, Kani solved 3, 6 and 9 and VeriFast solved 5 (arXiv:2606.17374, arXiv:2510.01072).

### 6. Case studies
- **SymCrypt (Microsoft):** SHA-3 and ML-KEM extracted to Lean 4, with AI-written proofs checked by the kernel.
- **Cedar (AWS):** the Rust authorizer is tested against a Lean model with millions of cargo-fuzz inputs; proofs found 4 validator bugs and differential and property testing 21 more (arXiv:2407.01688).
- **Verus:** the verified-nrkernel page table, VeriSMo, and storage and distributed systems.
- **hax:** Cryspen's ML-KEM in F* and Lean; Plonky3 and RISC Zero in Lean 4.
- **RefinedRust:** IBM's ACE security monitor, unsafe code included.

## Recommendations
1. **Kernel-checked Lean 4 proofs now:** apply Charon and Aeneas, with the Lean backend, to the safe, ownership-respecting part of your code, the most mature Rust-to-Lean route in 2026. Expect toolchain drift and gaps around unsafe code, and let AI provers handle routine goals, since the kernel re-checks them.
2. **Automation within Rust:** choose Verus for pointer- or concurrency-heavy systems code, Creusot for algorithms and data structures, or Prusti for cheap overflow and panic checks. When the solver times out or unsafe reasoning is needed, move to RefinedRust or Aeneas.
3. **Push-button checks, including unsafe code:** start with Kani, which needs no annotations for memory safety and panics, and add contracts only where bounded proofs fall short.
4. **Free, always-on guarantees:** use typestate, sealed traits, ghost tokens, nutype and static_assertions until arithmetic or quantified reasoning is needed.
5. **Proof tooling in Rust:** use `z3` or `easy-smt`, `egg` or `egglog`, study `nanoda_lib`, and read MIR through Charon or StableMIR.
6. **Follow the contracts work:** native contracts in rustc would give all these tools one specification source.

## Caveats
- Aeneas, hax, RustHorn, Prusti and Creusot assume safe Rust; only RefinedRust, Verus and Kani reason meaningfully about unsafe code today.
- SMT tools trust their solver and encoding, and some trust the borrow checker; foundational approaches trust less but need more manual proof.
- Maturity varies: most tools named here are active in 2026, but Chalk has been absorbed into rustc, rsmt2 looks dormant and `generic_const_exprs` is unfinished.
- A kernel checks AI-written proofs, but no kernel catches a wrong specification, so specifications still need expert review.
- Version numbers here date from September 2026 and move fast; check them before pinning.
- A few secondary details, such as Kani's newest release, could not be confirmed beyond the cited sources.
