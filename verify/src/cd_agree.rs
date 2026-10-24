// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Bounded agreement between the Verus-restated codecs (`s2rst-deductive`) and the library
//! (spec 003 FR-011; research R12; contracts/deductive-conventions.md, "Links").
//!
//! The Verus proofs hold for the restated copies. These harnesses tie each copy to the library
//! function it mirrors, within the R5 bounds:
//! - **decoders** `a_<codec>_dec__b<L>`: on every `L`-byte input, the library on `&mut &b[..]`
//!   and the restated decoder on `(&b, 0)` agree on acceptance, on the value and on the bytes
//!   consumed;
//! - **encoders** `a_<codec>_enc__n<k>`: for every sequence of `k` values, both write the same
//!   bytes.
//!
//! The string vector has no decoder harness: the library's decoder is a tool limit (T029), so
//! its link is the tested agreement alone (`verify/deductive/tests/agree.rs`; FR-011's
//! fallback, recorded in `verify/TRUSTED_BASE.md`). Its encoder is linked here. The varint has
//! no harness of its own: the library's `read_uvarint` and `write_uvarint` are crate-private,
//! and every vector harness here reads or writes a header with them.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::CellId;
use s2rst::s2::encoded_s2cell_id_vector as lib_cid;
use s2rst::s2::encoded_string_vector as lib_str;
use s2rst::s2::encoded_uint_vector as lib_uv;
use s2rst_deductive::{cellid_vector, fixed_width, string_vector, uint_vector};

use crate::codec::{same_bytes, same_slices};
use crate::wire_domain::{any_ids, any_strings, any_u32s, any_u64s, discard, with_bytes};

/// Both decoders' answers agree: acceptance, the value (element by element over `MAX`) and
/// the bytes consumed.
fn agree<T: PartialEq, const MAX: usize>(
    lib: std::io::Result<Vec<T>>,
    lib_consumed: usize,
    restated: Option<(Vec<T>, usize)>,
) -> bool {
    let accepted = lib.is_ok();
    match (lib, restated) {
        (Ok(a), Some((b, n))) => {
            assert!(
                same_slices::<MAX, T>(&a, &b, |x, y| x == y),
                "a different value"
            );
            assert!(lib_consumed == n, "a different number of bytes consumed");
        }
        (Err(e), None) => discard::<()>(Err(e)),
        (Ok(_), None) => panic!("the library accepts an input the restated decoder rejects"),
        (Err(e), Some(_)) => {
            discard::<()>(Err(e));
            panic!("the library rejects an input the restated decoder accepts");
        }
    }
    accepted
}

/// One decoder member per `name = length, unwind`.
macro_rules! decoder {
    ($what:literal, $lib:expr, $restated:expr, $map:expr, $min:literal; $($(#[$attr:meta])* $name:ident = $len:literal, $unwind:literal),* $(,)?) => { $(
        #[doc = concat!("Bounded agreement (model check), ", $what, ": on every input of exactly ", stringify!($len), " bytes, the library and the restated decoder agree (FR-011).")]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            with_bytes::<$len>(|b| {
                let mut input = b;
                let lib = $lib(&mut input).map($map);
                let consumed = $len - input.len();
                let accepted = agree::<_, $len>(lib, consumed, $restated(b, 0));
                kani::cover!(accepted || $len < $min, "accepted (inputs shorter than the format's minimum cannot be)");
                kani::cover!(!accepted, "rejected");
            });
        }
    )* };
}

/// One encoder member per `name = count, encoding bound, unwind`.
macro_rules! encoder {
    ($what:literal, $gen:ident, $lib:expr, $restated:expr; $($(#[$attr:meta])* $name:ident = $n:literal, $e:literal, $unwind:literal),* $(,)?) => { $(
        #[doc = concat!("Bounded agreement (model check), ", $what, ": for every sequence of exactly ", stringify!($n), " values, the library and the restated encoder write the same bytes (FR-011).")]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            let v = $gen::<$n>();
            let (mut a, mut b) = (Vec::with_capacity(64), Vec::with_capacity(64));
            ($lib)(&v, &mut a);
            ($restated)(&v, &mut b);
            assert!(same_bytes::<$e>(&a, &b), "the encodings differ");
            kani::cover!(!a.is_empty(), "written");
        }
    )* };
}

/// The ids' bits. The vector grows on demand: a capacity from a decoded length is the memory
/// wall of research R0 (finding 2).
fn ids_of(v: Vec<CellId>) -> Vec<u64> {
    let mut out = Vec::new();
    for c in v {
        out.push(c.0);
    }
    out
}

decoder!("decode_uint_vector_u64", lib_uv::decode_uint_vector_u64, uint_vector::decode_uint_vector_u64, |v| v, 1;
    a_uv64_dec__b0 = 0, 9,
    a_uv64_dec__b1 = 1, 9,
    a_uv64_dec__b2 = 2, 9,
    a_uv64_dec__b3 = 3, 9,
    a_uv64_dec__b4 = 4, 9,
    a_uv64_dec__b5 = 5, 9,
    a_uv64_dec__b6 = 6, 9,
    a_uv64_dec__b7 = 7, 9,
    a_uv64_dec__b8 = 8, 9,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_uv64_dec__b9 = 9, 10,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_uv64_dec__b10 = 10, 11,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_uv64_dec__b11 = 11, 12,
);

decoder!("decode_uint_vector_u32", lib_uv::decode_uint_vector_u32, uint_vector::decode_uint_vector_u32, |v| v, 1;
    a_uv32_dec__b0 = 0, 9,
    a_uv32_dec__b1 = 1, 9,
    a_uv32_dec__b2 = 2, 9,
    a_uv32_dec__b3 = 3, 9,
    a_uv32_dec__b4 = 4, 9,
    a_uv32_dec__b5 = 5, 9,
    a_uv32_dec__b6 = 6, 9,
    a_uv32_dec__b7 = 7, 9,
    a_uv32_dec__b8 = 8, 9,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_uv32_dec__b9 = 9, 10,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_uv32_dec__b10 = 10, 11,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_uv32_dec__b11 = 11, 12,
);

decoder!("decode_s2cell_id_vector", lib_cid::decode_s2cell_id_vector, cellid_vector::decode_s2cell_id_vector, ids_of, 2;
    a_cv_dec__b0 = 0, 9,
    a_cv_dec__b1 = 1, 9,
    a_cv_dec__b2 = 2, 9,
    a_cv_dec__b3 = 3, 9,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_cv_dec__b4 = 4, 9,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_cv_dec__b5 = 5, 9,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_cv_dec__b6 = 6, 9,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_cv_dec__b7 = 7, 9,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_cv_dec__b8 = 8, 9,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_cv_dec__b9 = 9, 10,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_cv_dec__b10 = 10, 11,
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T058)
    a_cv_dec__b11 = 11, 12,
);

encoder!("encode_uint_vector_u64", any_u64s,
    |v: &Vec<u64>, w: &mut Vec<u8>| { assert!(lib_uv::encode_uint_vector_u64(v, w).is_ok()); },
    |v: &Vec<u64>, w: &mut Vec<u8>| { let _ = uint_vector::encode_uint_vector_u64(v, w); };
    a_uv64_enc__n0 = 0, 1, 11,
    #[cfg(feature = "slow")] // weekly: out of memory at 4 GiB (T058); the restated encoder appends byte by byte
    a_uv64_enc__n1 = 1, 9, 11,
    // `__n2`: out of memory at 8 GiB too (T058), `cd_xlarge::a_uv64_enc__n2`.
);

encoder!("encode_uint_vector_u32", any_u32s,
    |v: &Vec<u32>, w: &mut Vec<u8>| { assert!(lib_uv::encode_uint_vector_u32(v, w).is_ok()); },
    |v: &Vec<u32>, w: &mut Vec<u8>| { let _ = uint_vector::encode_uint_vector_u32(v, w); };
    a_uv32_enc__n0 = 0, 1, 11,
    #[cfg(feature = "slow")] // weekly: out of memory at 4 GiB (T058); the restated encoder appends byte by byte
    a_uv32_enc__n1 = 1, 5, 11,
    // `__n2`: out of memory at 8 GiB too (T058), `cd_xlarge::a_uv32_enc__n2`.
);

encoder!("encode_s2cell_id_vector", any_ids,
    |v: &Vec<CellId>, w: &mut Vec<u8>| { assert!(lib_cid::encode_s2cell_id_vector(v, w).is_ok()); },
    |v: &Vec<CellId>, w: &mut Vec<u8>| cellid_vector::encode_s2cell_id_vector(&ids_of(v.clone()), w);
    a_cv_enc__n0 = 0, 2, 11,
);

/// Bounded agreement (model check), `encode_string_vector`: strings of up to 2 bytes (research R5).
macro_rules! string_encoder {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!("Bounded agreement (model check), encode_string_vector: for every sequence of exactly ", stringify!($n), " strings of up to 2 bytes, the library and the restated encoder write the same bytes (FR-011).")]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        fn $name() {
            let v = any_strings::<$n, 2>();
            let mut refs: Vec<&[u8]> = Vec::with_capacity($n);
            for s in &v {
                refs.push(s.as_slice());
            }
            let (mut a, mut b) = (Vec::with_capacity(64), Vec::with_capacity(64));
            assert!(lib_str::encode_string_vector(&refs, &mut a).is_ok());
            let _ = string_vector::encode_string_vector(&v, &mut b);
            assert!(same_bytes::<8>(&a, &b), "the encodings differ");
            kani::cover!(!a.is_empty(), "written");
        }
    )* };
}

string_encoder!(a_sv_enc__n0 = 0, a_sv_enc__n1 = 1);
// `__n2`: out of memory at 4 and 8 GiB (T058), `cd_xlarge::a_sv_enc__n2`.

/// Bounded agreement (model check), `encode_uint_with_length` and `get_uint_with_length`: for every
/// value, every `len ≤ 8` and every 8 bytes of data, the library and the restated helpers agree.
#[kani::proof]
#[kani::unwind(10)]
fn a_fw() {
    let v: u64 = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 8);
    let (mut a, mut b) = (Vec::with_capacity(8), Vec::with_capacity(8));
    assert!(lib_uv::encode_uint_with_length(&mut a, v, len).is_ok());
    fixed_width::encode_uint_with_length(&mut b, v, len);
    assert!(same_bytes::<8>(&a, &b), "encode_uint_with_length differs");
    with_bytes::<8>(|d| {
        assert!(
            lib_uv::get_uint_with_length(d, len) == fixed_width::get_uint_with_length(d, len),
            "get_uint_with_length differs"
        );
    });
    kani::cover!(len == 8, "eight bytes");
}

/// One `decode_uint_with_length` member per `name = length => |result, len| covers`.
macro_rules! fixed_width_decoder {
    ($($name:ident = $len:literal => |$lib:ident, $n:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!("Bounded agreement (model check), decode_uint_with_length: on every input of exactly ", stringify!($len), " bytes and every `len ≤ 8`, the library and the restated decoder agree (FR-011).")]
        #[kani::proof]
        #[kani::unwind(10)]
        fn $name() {
            let $n: usize = kani::any();
            kani::assume($n <= 8);
            with_bytes::<$len>(|d| {
                let mut input = d;
                let $lib = lib_uv::decode_uint_with_length(&mut input, $n).map(|x| (x, $len - input.len()));
                let restated = fixed_width::decode_uint_with_length(d, 0, $n);
                assert!($lib.as_ref().ok() == restated.as_ref(), "decode_uint_with_length differs");
                let () = $covers;
                discard($lib);
            });
        }
    )* };
}

fixed_width_decoder!(
    a_fw_dec__b0 = 0 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_err(), "too short");
    },
    a_fw_dec__b1 = 1 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_err(), "too short");
    },
    a_fw_dec__b2 = 2 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_err(), "too short");
    },
    a_fw_dec__b3 = 3 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_err(), "too short");
    },
    a_fw_dec__b4 = 4 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_err(), "too short");
    },
    a_fw_dec__b5 = 5 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_err(), "too short");
    },
    a_fw_dec__b6 = 6 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_err(), "too short");
    },
    a_fw_dec__b7 = 7 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_err(), "too short");
    },
    a_fw_dec__b8 = 8 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_ok() && len == 8, "eight bytes read");
    },
    a_fw_dec__b9 = 9 => |lib, len| {
        kani::cover!(lib.is_ok() && len == 0, "zero bytes");
        kani::cover!(lib.is_ok() && len == 8, "eight bytes read");
    },
);
