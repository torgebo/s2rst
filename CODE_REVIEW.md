# Code review: changes since v0.4.0

Reviewed 2026-10-11: the 3 commits after `v0.4.0` (263fad3, bbe6f4e, 4a4e78d). The
reviewer also ran `cargo check` on the whole workspace and the new tests, and all passed.
#1–#4 were re-checked against the code by hand.

## Bugs

| # | Where | Problem |
|---|---|---|
| 1 | `.github/workflows/fuzz.yml:47` | The new fuzz targets `decode_cell_id`, `decode_point` and `decode_rect` point at seed folders that were never committed (only `parse_cell_id_text` got seeds). Those 3 nightly jobs will fail every night without fuzzing anything. |
| 2 | `core/src/s2/cell_union.rs:505` | `denormalize` with `level_mod` above 3 now expands every cell down to level 30 instead of panicking. The Python binding passes `level_mod` through unchecked, so `denormalize(0, 200)` hangs and runs out of memory where v0.4.0 raised an error. |
| 3 | `core/src/s2/density_tree.rs:1318` | The varint fix (reject a tenth byte above 1) wasn't applied to the density tree's own varint decoder. It still silently drops the high bits, so the same bad input is rejected in one place and accepted in another. |
| 4 | `.github/workflows/verify.yml:205` | CI's weekly decoding job runs 2 harnesses at once, each allowed 8 GiB, on a 16 GB runner, and the cgroup memory guard probably doesn't work on GitHub's runners. |
| 5 | `verify/deductive/run-verus.sh:82` | In mutant mode, any failure of Verus counts as "killed". If Verus fails to start, runs out of memory or doesn't compile the patch, the non-vacuity check passes without testing anything. |

## Behaviour changes to decide on

- **6, `CellUnion::decode`:** it no longer normalises, and it rejects unsorted or
  overlapping ids. Encodings with unsorted ids from C++ or Go, which v0.4.0 accepted and
  normalised, now fail. The Python docstring still says decoded unions are "automatically
  normalized".
- **8, `from_range`:** it now panics in debug builds on arguments that v0.4.0 accepted
  (out-of-order or non-leaf cells). The Python and WASM bindings don't check for these.
- **9, `CellUnion` docs:** they say only `from_verbatim` can build an invalid union, but
  deserializing JSON (with the `serde` feature) can too.

## Smaller issues

- **7:** changing a constant such as `MAX_DECODE_COUNT` doesn't trigger the guard that keeps
  the Verus copies in step with the library.
- **10:** pressing Ctrl-C during `run-mutants.sh` deletes the worktrees, but the loop keeps
  going and prints misleading errors.
- **11:** `reconstruct_uvarint` is a second, unverified copy of `read_uvarint`.
- **12–13:** some decoders dropped up-front memory reservation even where the length is
  already known safe, or could be capped. Large valid inputs now pay for repeated regrowth.
- **14:** two dead branches remain in `from_token`, plus a dead `shift >= 64` check in the
  varint readers.
- **15:** `from_range` is now just a second name for `from_begin_end`.

## Note on #4

The simple fix, one decoding harness at a time on CI, has a cost. That part took 3.8 hours
one at a time on the workstation, CI runners are slower, and CI's slow job has a 6-hour
limit. So it may need splitting into two CI jobs rather than just setting `VERIFY_JOBS=1`.

## Status (2026-10-11)

- **#1 fixed.** Seeds added under `fuzz/seeds/{decode_cell_id,decode_point,decode_rect}/`
  (valid values plus one rejected id); each decodes as intended, and a 30-second run of each
  target with them found nothing. The four new targets are in `fuzz/README.md`'s table.
- **#2 fixed in the binding.** Python `CellUnion.denormalize` raises `ValueError` unless
  `min_level` is 0–30 and `level_mod` is 1–3. The core keeps upstream's `DCHECK` as a
  `debug_assert!`, and its docs now say what a larger `level_mod` does in release builds.
- **#3 fixed.** `density_tree::decode_varint_at` rejects a tenth byte above 1, as upstream
  and the other two varint readers do; regression test added.
- **#4 fixed.** The weekly decoding leg is now three legs (`F=decoding P=vectors|links|codecs`),
  each one harness at a time. `make verify-slow` takes `P` with `F=decoding`.
- **#5 fixed.** A Verus mutant counts as killed only if Verus reports a verification error;
  one it cannot check is an `ERROR` and fails the run. Checked with a mutant that does not
  compile (now `ERROR`, before `killed`) and a comment-only one (`SURVIVED`); the ten real
  mutants are all still killed.
- **#8 kept as C++.** `from_range`/`from_begin_end`/`from_min_max` keep upstream's
  preconditions as debug assertions. A release build with `CellId(0)` as `begin` loops
  forever (as C++ does), so the Python and WASM bindings now check the preconditions and
  raise `ValueError` / throw.
- **#6 kept (option A).** `CellUnion::decode` keeps the ids as encoded, as C++ does, and
  returns an error (no panic) for ids that are not a valid union, where C++ accepts anything.
  The Python and WASM docs no longer say decoded unions are normalised.
- **#9 fixed (option A).** With the `serde` feature, `Deserialize` is hand-written: it reads
  the same form as before and rejects what binary decoding rejects, keeping valid ids as
  given. `Serialize` is still derived, so the format is unchanged. No public `TryFrom` was
  added (it would be a new operation for `verify/cell_union/INVENTORY.md`), and the inventory
  notes the new impl.
- **#7 fixed.** The drift guard's fingerprint now also covers the module constants a mirrored
  function names (top-level `const` items of its file), so changing `MAX_DECODE_COUNT` trips
  it (checked by changing it to 100,000,000). The two affected fingerprints were re-blessed;
  the Verus copy has the same value.
- **#10 fixed.** `run-mutants.sh`: Ctrl-C or TERM now exits (status 130/143), and the EXIT
  trap cleans up once, instead of the loop carrying on in removed worktrees.
- **#11 fixed.** `reconstruct_uvarint` calls the verified `read_uvarint` over the first byte
  chained with the rest of the input, instead of copying it. A test for the `80 80` header
  that the mutation measurement found missing was added too.
- **#12 and #13: not changed in code, by design.** Spec 003 research R7 tried a capped
  reservation, and any reservation sized from decoded data (even a known `len()`) kept the
  decoders out of CBMC's reach; growth on demand cost within 3 % in the benchmarks. The
  comments at each site now give that reason, so the choice is not mistaken for an oversight.
- **#14 fixed.** `from_token` parses in one pass over the bytes (as the spec model does), so
  the redundant `"X"` check and the unreachable `from_str_radix` failure are gone. The dead
  `shift >= 64` check is gone from `read_uvarint` and from its Verus copy (Verus proves the
  shift stays at most 63 without it).
- **#15 fixed.** `CellUnion::from_range` is deprecated in favour of `from_begin_end` (the C++
  name). Python keeps `from_range` (it has no `from_begin_end`) and, like WASM `fromRange`,
  calls `from_begin_end`.
- **Also found:** mutant `M-055` no longer applied to HEAD (263fad3 rewrote its context); it
  is regenerated and killed again. `M-050`, `M-216` and `M-250` were regenerated for the
  `from_token` and `read_uvarint` changes and are killed.
