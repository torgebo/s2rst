// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Formal verification of `s2rst::s2::CellId` (spec 001), `s2rst::s2::CellUnion`
//! (spec 002), the decoding layer (spec 003) and the value types `s2rst::s2::Cell` and
//! `s2rst::s1::Interval` (spec 004), `s2rst::r1::Interval` and `s2rst::s2::Rect` (spec 005),
//! `s2rst::r2::Rect` and `s2rst::s2::Point` (spec 006), `s2rst::s1::ChordAngle` and
//! `s2rst::s1::Angle` (spec 007), `s2rst::s2::Cap` (spec 008), and `s2rst::s2::LatLng` with
//! `s2rst::s2::Rect` as a `Region` (spec 010).
//!
//! - [`spec`] is the reference model of the S2 cell hierarchy, written from upstream
//!   `s2cell_id.h` / `s2coords.h` and independent of `core/src/s2/cell_id.rs` (FR-006). It is
//!   ordinary Rust, sanity-tested against the committed C++ oracle vectors by `cargo test`.
//! - The `#[cfg(kani)]` modules are Kani proof harnesses: `crash` (crash freedom, FR-003) and
//!   one module per property area (FR-005). Their results are recorded in `INVENTORY.md`
//!   (the FR-002 operation inventory) and `PROPERTIES.md` (the FR-005 property catalogue).
//! - [`union_spec`] is the reference model of a cell union, the set of leaf cells it covers,
//!   written from upstream `s2cell_union.h` / `.cc` on top of [`spec`]. The `cu_*` modules
//!   are the `CellUnion` harnesses, behind the default `cellunion` feature. Their results
//!   are recorded in `cell_union/INVENTORY.md` and `cell_union/PROPERTIES.md`.
//! - [`wire_spec`] is the reference description of the in-scope wire formats, written from
//!   upstream's format code (spec 003). The `cd_*` modules are the decoding-layer harnesses,
//!   behind the default `decoding` feature, except `cd_xlarge`, the members over the weekly
//!   budget, behind `xlarge` (`make verify-xlarge`, on a machine with 32 GB of memory).
//!   Their results are recorded in `decoding/INVENTORY.md` and `decoding/PROPERTIES.md`.
//! - [`cell_spec`] is the reference description of a cell, on top of [`spec`], with
//!   upstream's u-v projection; [`arc_spec`] describes a circular interval as the set of
//!   points it denotes (spec 004). The `cl_*` (`Cell`) and `iv_*` (`s1::Interval`) modules are
//!   their harnesses, behind the default `values` feature; `laws` states the generic trait
//!   laws once for every suite. Their results are recorded in `values/INVENTORY.md` and
//!   `values/PROPERTIES.md`.
//! - [`real_spec`] describes a real interval as the closed set of numbers between its bounds;
//!   [`rect_spec`] describes a latitude–longitude rectangle as the product of that and
//!   [`arc_spec`] (spec 005). The `ri_*` (`r1::Interval`) and `rl_*` (`s2::Rect`) modules are
//!   their harnesses, in the same `values` suite and records.
//! - [`uv_spec`] describes a u-v rectangle as the product of two [`real_spec`] sets, and
//!   [`point_spec`] describes the point's order, classifications and scaling, with
//!   restatements of upstream's arithmetic kept apart (spec 006). The `ur_*` (`r2::Rect`) and
//!   `pt_*` (`s2::Point`) modules are their harnesses, in the same `values` suite and records.
//! - [`chord_spec`] describes a chord angle by its canonical values, their order and their
//!   neighbours, and [`angle_spec`] restates upstream's angle conversions and E-forms (spec
//!   007). The `ca_*` (`s1::ChordAngle`) and `ag_*` (`s1::Angle`) modules are their harnesses,
//!   in the same `values` suite and records; `tests/angle_enumeration.rs` proves the E-form
//!   round trips by enumerating every `i32` (spec 007 research R12).
//! - [`cap_spec`] describes a cap as the points within a chord distance of its centre, with
//!   restatements of upstream's arithmetic kept apart (spec 008). The `cp_*` (`s2::Cap`)
//!   modules are its harnesses, in the same `values` suite and records. `stubs` holds the only
//!   replacements of library code in any proof: the function contracts of `asin`, `atan2` and
//!   `cos`, and the lemma stubs, each resting on lemmas proven on the real code (spec 008
//!   FR-003, FR-003a; research R5, R6); `tests/libm_contracts.rs` checks the contracts against
//!   the real functions.
//! - [`latlng_spec`] describes a latitude–longitude as a pair of angles, valid in the principal
//!   ranges, with upstream's conversions restated apart and a copy of the exact reduction's
//!   correction step (spec 010). The `ll_*` (`s2::LatLng`) modules are its harnesses, and
//!   `rl_reg` instantiates the `Region` laws for `s2::Rect`; `stubs` gains pure contracts (one
//!   contract value per input) and a contract on `Cell::rect_bound` (spec 010 research R5, R6).
//!
//! Every property harness's doc comment starts with its catalogue id and its kind, then a
//! plain-language statement, then the exact one: `P-HIER-01 (correct result): …`. The kinds
//! are `no crash`, `correct result`, `round trip`, `exact output`, `exact input`, `validity`,
//! `law` and `model check`, so `grep -rn '(round trip)' src` lists every round-trip proof.
//! `PROPERTIES_BY_KIND.md` lists them all by kind; crash-freedom harnesses start with their
//! obligation, `C-<operation>:`.
//!
//! Run them with `make verify` from the repository root; never run `cargo kani` uncapped
//! (see `README.md`).

#![deny(unsafe_code)]

// The proofs and the restatements of upstream's arithmetic (`point_spec`, `cap_spec`) describe
// the library built without the `fma` target feature, as Kani builds it: with it, `r3::Vector`'s
// `dot` and `cross` fuse their products (`core/src/r3/vector.rs`), and every "bit for bit
// upstream's" statement would describe other code (TRUSTED_BASE.md, "The build"; code review of
// v0.4.0..74b9bc1, finding 8).
#[cfg(target_feature = "fma")]
compile_error!(
    "s2rst-verify describes the library built without the `fma` target feature; build it \
     without `-C target-cpu=native` or `-C target-feature=+fma`"
);

pub mod angle_spec;
pub mod arc_spec;
pub mod cap_spec;
pub mod cell_spec;
pub mod chord_spec;
pub mod latlng_spec;
pub mod point_spec;
pub mod real_spec;
pub mod rect_spec;
pub mod spec;
pub mod union_spec;
pub mod uv_spec;
pub mod wire_spec;

#[cfg(kani)]
mod domain;
// The generic trait laws (spec 004 research R15): every suite may instantiate them.
#[cfg(kani)]
mod laws;
// Function contracts and lemma stubs (spec 008 research R5, R6): every `#[kani::stub]` names
// one of these.
#[cfg(kani)]
mod stubs;

#[cfg(kani)]
mod anc;
#[cfg(kani)]
mod crash;
#[cfg(kani)]
mod enc;
#[cfg(kani)]
mod hier;
// The neighbour properties are all in the weekly slow tier (feature `slow`).
#[cfg(all(kani, feature = "slow"))]
mod nbr;
#[cfg(kani)]
mod range;
#[cfg(kani)]
mod trav;
#[cfg(kani)]
mod val;

#[cfg(all(kani, feature = "cellunion"))]
mod union_domain;

#[cfg(all(kani, feature = "cellunion"))]
mod cu_acc;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_crash;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_den;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_enc;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_inv;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_meas;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_mem;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_norm;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_range;
#[cfg(all(kani, feature = "cellunion"))]
mod cu_set;

#[cfg(all(kani, feature = "decoding"))]
mod codec;
#[cfg(all(kani, feature = "decoding"))]
mod wire_domain;

#[cfg(all(kani, feature = "decoding"))]
mod cd_agree;
#[cfg(all(kani, feature = "decoding"))]
mod cd_crash;
#[cfg(all(kani, feature = "decoding"))]
mod cd_cv;
#[cfg(all(kani, feature = "decoding"))]
mod cd_fw;
#[cfg(all(kani, feature = "decoding"))]
mod cd_fx;
#[cfg(all(kani, feature = "decoding"))]
mod cd_pv;
#[cfg(all(kani, feature = "decoding"))]
mod cd_sv;
#[cfg(all(kani, feature = "decoding"))]
mod cd_uv;
#[cfg(all(kani, feature = "decoding", feature = "xlarge"))]
mod cd_xlarge;

#[cfg(all(kani, feature = "values"))]
mod value_domain;

#[cfg(all(kani, feature = "values"))]
mod cl_bnd;
#[cfg(all(kani, feature = "values"))]
mod cl_chd;
#[cfg(all(kani, feature = "values"))]
mod cl_cnt;
#[cfg(all(kani, feature = "values"))]
mod cl_crash;
#[cfg(all(kani, feature = "values"))]
mod cl_eq;
#[cfg(all(kani, feature = "values"))]
mod cl_id;

#[cfg(all(kani, feature = "values"))]
mod iv_crash;
#[cfg(all(kani, feature = "values"))]
mod iv_exp;
#[cfg(all(kani, feature = "values"))]
mod iv_mea;
#[cfg(all(kani, feature = "values"))]
mod iv_mem;
#[cfg(all(kani, feature = "values"))]
mod iv_rel;
#[cfg(all(kani, feature = "values"))]
mod iv_set;
#[cfg(all(kani, feature = "values"))]
mod iv_val;

#[cfg(all(kani, feature = "values"))]
mod ri_crash;
#[cfg(all(kani, feature = "values"))]
mod ri_exp;
#[cfg(all(kani, feature = "values"))]
mod ri_mea;
#[cfg(all(kani, feature = "values"))]
mod ri_mem;
#[cfg(all(kani, feature = "values"))]
mod ri_rel;
#[cfg(all(kani, feature = "values"))]
mod ri_set;
#[cfg(all(kani, feature = "values"))]
mod ri_val;

// P-LBND-01, its only harness, is in the weekly slow tier (feature `slow`).
#[cfg(all(kani, feature = "values", feature = "slow"))]
mod rl_bnd;
#[cfg(all(kani, feature = "values"))]
mod rl_crash;
#[cfg(all(kani, feature = "values"))]
mod rl_eq;
#[cfg(all(kani, feature = "values"))]
mod rl_exp;
#[cfg(all(kani, feature = "values"))]
mod rl_mem;
#[cfg(all(kani, feature = "values"))]
mod rl_reg;
#[cfg(all(kani, feature = "values"))]
mod rl_rel;
#[cfg(all(kani, feature = "values"))]
mod rl_set;
#[cfg(all(kani, feature = "values"))]
mod rl_val;

#[cfg(all(kani, feature = "values"))]
mod ur_crash;
#[cfg(all(kani, feature = "values"))]
mod ur_eq;
#[cfg(all(kani, feature = "values"))]
mod ur_exp;
#[cfg(all(kani, feature = "values"))]
mod ur_mea;
#[cfg(all(kani, feature = "values"))]
mod ur_mem;
#[cfg(all(kani, feature = "values"))]
mod ur_rel;
#[cfg(all(kani, feature = "values"))]
mod ur_set;
#[cfg(all(kani, feature = "values"))]
mod ur_val;

#[cfg(all(kani, feature = "values"))]
mod pt_ari;
#[cfg(all(kani, feature = "values"))]
mod pt_cls;
#[cfg(all(kani, feature = "values"))]
mod pt_crash;
#[cfg(all(kani, feature = "values"))]
mod pt_dst;
#[cfg(all(kani, feature = "values"))]
mod pt_ord;
#[cfg(all(kani, feature = "values"))]
mod pt_ort;
#[cfg(all(kani, feature = "values"))]
mod pt_scl;
#[cfg(all(kani, feature = "values"))]
mod pt_val;

#[cfg(all(kani, feature = "values"))]
mod ca_ari;
#[cfg(all(kani, feature = "values"))]
mod ca_cls;
#[cfg(all(kani, feature = "values"))]
mod ca_crash;
#[cfg(all(kani, feature = "values"))]
mod ca_err;
#[cfg(all(kani, feature = "values"))]
mod ca_nbr;
#[cfg(all(kani, feature = "values"))]
mod ca_ord;
#[cfg(all(kani, feature = "values"))]
mod ca_val;

#[cfg(all(kani, feature = "values"))]
mod ag_ari;
#[cfg(all(kani, feature = "values"))]
mod ag_cnv;
#[cfg(all(kani, feature = "values"))]
mod ag_crash;
#[cfg(all(kani, feature = "values"))]
mod ag_eform;
#[cfg(all(kani, feature = "values"))]
mod ag_val;

#[cfg(all(kani, feature = "values"))]
mod cp_bnd;
#[cfg(all(kani, feature = "values"))]
mod cp_cls;
#[cfg(all(kani, feature = "values"))]
mod cp_cmp;
#[cfg(all(kani, feature = "values"))]
mod cp_con;
#[cfg(all(kani, feature = "values"))]
mod cp_crash;
#[cfg(all(kani, feature = "values"))]
mod cp_eq;
#[cfg(all(kani, feature = "values"))]
mod cp_gro;
#[cfg(all(kani, feature = "values"))]
mod cp_lem;
#[cfg(all(kani, feature = "values"))]
mod cp_mea;
#[cfg(all(kani, feature = "values"))]
mod cp_mem;
#[cfg(all(kani, feature = "values"))]
mod cp_reg;
#[cfg(all(kani, feature = "values"))]
mod cp_rel;

// Weekly, as `rl_bnd`: both harnesses take about eight minutes (spec 010, 2026-10-28).
#[cfg(all(kani, feature = "values", feature = "slow"))]
mod ll_apx;
#[cfg(all(kani, feature = "values"))]
mod ll_ari;
#[cfg(all(kani, feature = "values"))]
mod ll_con;
#[cfg(all(kani, feature = "values"))]
mod ll_crash;
#[cfg(all(kani, feature = "values"))]
mod ll_dst;
#[cfg(all(kani, feature = "values"))]
mod ll_eq;
#[cfg(all(kani, feature = "values"))]
mod ll_fp;
#[cfg(all(kani, feature = "values"))]
mod ll_lem;
#[cfg(all(kani, feature = "values"))]
mod ll_nrm;
#[cfg(all(kani, feature = "values"))]
mod ll_tp;
#[cfg(all(kani, feature = "values"))]
mod ll_val;
