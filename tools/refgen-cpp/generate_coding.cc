// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
//
// Reference-vector generator for the s2rst decoding layer, using the C++ reference
// implementation (google/s2geometry) as the oracle (specs/003-formal-verify-decoders, FR-024,
// research R10, data-model §10). Emits CSV that core/tests/cpp_coding_diff.rs diffs the Rust
// codecs against, and that verify/tests/wire_spec_vs_cpp.rs checks the reference description
// against. Deterministic (fixed seed). See tools/refgen-cpp/README.md.
//
// Columns: kind,input,expected,note.
//   - Byte strings are lowercase hex. Lists are ';'-separated; a point is "x:y:z" with each
//     coordinate the 16-hex-digit bit pattern of the double (a cap adds ":length2", a
//     rectangle is "lat_lo:lat_hi:lng_lo:lng_hi").
//   - Each distinct row is printed once (prefixes of different encodings coincide).
//   - ENCODE rows: input is the value(s), expected the encoded bytes. STRING_VECTOR_ENCODE's
//     input is `<count>|<items>`.
//   - DECODE rows: input is the bytes; expected is `reject`, or `ok|<consumed>|<count>|<items>`
//     (`<count>` is the number of decoded items, so that "no strings" and "one empty string"
//     differ).
//   - note is empty, or `deviation:reject` where the library documents that it rejects an
//     input C++ accepts (spec FR-022: invalid CellId and Cap values, and a NaN cap length²,
//     which C++ decodes as a full cap; D-004).
//   - or `deviation:negative-canonical` on a CAP_DECODE row whose length² C++ keeps as a
//     negative value other than -1: the library's ChordAngle::from_length2 makes every negative
//     value Negative() (-1), so it decodes the same empty cap with length² -1 (spec 007
//     FR-017, option B).
//
// Inputs deliberately left out:
//   - S2Cell::Decode of an invalid id: S2Cell's constructor then calls S2CellId::level() on
//     id 0 (U-001's ABSL_ASSUME, undefined behaviour) or indexes face tables with face 6 or 7.
//     That is candidate U-004 (specs/003-formal-verify-decoders tasks.md T074); the library
//     rejects such ids.
//   - Vectors above the library's count limits (50,000,000 elements or points, 500,000,000
//     string bytes): a row would be tens of megabytes. The limits are documented deviations
//     (FR-022) and unit-tested on the Rust side.

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <random>
#include <set>
#include <string>
#include <vector>

#include "absl/strings/string_view.h"
#include "absl/types/span.h"
#include "s2/encoded_s2cell_id_vector.h"
#include "s2/encoded_s2point_vector.h"
#include "s2/encoded_string_vector.h"
#include "s2/encoded_uint_vector.h"
#include "s2/s1angle.h"
#include "s2/s1chord_angle.h"
#include "s2/s2cap.h"
#include "s2/s2cell.h"
#include "s2/s2cell_id.h"
#include "s2/s2latlng.h"
#include "s2/s2latlng_rect.h"
#include "s2/s2point.h"
#include "s2/util/coding/coder.h"

namespace {

std::mt19937_64 rng(0x434F4449ULL);  // "CODI"
std::normal_distribution<double> gauss(0.0, 1.0);

// ---- formatting ---------------------------------------------------------------------------

std::string HexBytes(const std::string& s) {
  static const char kDigits[] = "0123456789abcdef";
  std::string out;
  for (unsigned char c : s) {
    out += kDigits[c >> 4];
    out += kDigits[c & 15];
  }
  return out;
}

std::string Hex64(uint64_t x) {
  char buf[17];
  std::snprintf(buf, sizeof buf, "%016llx", static_cast<unsigned long long>(x));
  return buf;
}

std::string Hex32(uint32_t x) {
  char buf[9];
  std::snprintf(buf, sizeof buf, "%08x", x);
  return buf;
}

uint64_t Bits(double d) {
  uint64_t b;
  std::memcpy(&b, &d, sizeof b);
  return b;
}

double FromBits(uint64_t b) {
  double d;
  std::memcpy(&d, &b, sizeof d);
  return d;
}

std::string PointText(const S2Point& p) {
  return Hex64(Bits(p.x())) + ":" + Hex64(Bits(p.y())) + ":" + Hex64(Bits(p.z()));
}

template <class T, class F>
std::string Join(const std::vector<T>& v, F f) {
  std::string out;
  for (size_t i = 0; i < v.size(); ++i) {
    if (i > 0) out += ';';
    out += f(v[i]);
  }
  return out;
}

std::string Bytes(const Encoder& e) { return std::string(e.base(), e.length()); }

std::set<std::string> printed;

void Row(const char* kind, const std::string& input, const std::string& expected,
         const char* note = "") {
  std::string row = std::string(kind) + "," + input + "," + expected + "," + note;
  if (printed.insert(row).second) std::printf("%s\n", row.c_str());
}

std::string Ok(size_t consumed, size_t count, const std::string& items) {
  return "ok|" + std::to_string(consumed) + "|" + std::to_string(count) + "|" + items;
}

// Every proper prefix of `s`, then `s` itself.
std::vector<std::string> WithPrefixes(const std::string& s) {
  std::vector<std::string> out;
  for (size_t k = 0; k <= s.size(); ++k) out.push_back(s.substr(0, k));
  return out;
}

S2Point RandUnit() {
  return S2Point(gauss(rng), gauss(rng), gauss(rng)).Normalize();
}

// ---- varint ---------------------------------------------------------------------------------

void VarintDecode(const std::string& in) {
  Decoder d(in.data(), in.size());
  uint64_t v = 0;
  if (!d.get_varint64(&v)) {
    Row("VARINT_DECODE", HexBytes(in), "reject");
  } else {
    Row("VARINT_DECODE", HexBytes(in), Ok(in.size() - d.avail(), 1, Hex64(v)));
  }
}

std::string Varint(uint64_t v) {
  Encoder e;
  e.Ensure(Varint::kMax64);
  e.put_varint64(v);
  return Bytes(e);
}

void Varints() {
  const uint64_t values[] = {0, 1, 127, 128, 300, uint64_t{1} << 32, uint64_t{1} << 63,
                             std::numeric_limits<uint64_t>::max()};
  for (uint64_t v : values) {
    for (const std::string& s : WithPrefixes(Varint(v))) VarintDecode(s);
    VarintDecode(Varint(v) + "\x05");  // a trailing byte is left unread
  }
  // Over-long encodings of 0 and 127.
  VarintDecode(std::string("\x80\x00", 2));
  VarintDecode(std::string("\xff\x80\x00", 3));
  // Ten-byte encodings: tenth byte 0, 1, 2 and 0x7f, alone and with a trailing byte (the
  // research R1 / D-003 question), and an eleventh continuation byte.
  const std::string nine(9, '\xff');
  for (unsigned char last : {0x00, 0x01, 0x02, 0x7f}) {
    std::string s = nine + static_cast<char>(last);
    VarintDecode(s);
    VarintDecode(s + std::string(1, '\0'));
  }
  VarintDecode(nine + "\x80\x01");
}

// ---- integer vectors --------------------------------------------------------------------------

template <class T>
std::string EncodeUints(const std::vector<T>& v) {
  Encoder e;
  s2coding::EncodeUintVector<T>(absl::MakeConstSpan(v), &e);
  return Bytes(e);
}

template <class T>
std::string UintText(T x);
template <>
std::string UintText<uint32_t>(uint32_t x) { return Hex32(x); }
template <>
std::string UintText<uint64_t>(uint64_t x) { return Hex64(x); }

template <class T>
void UintDecode(const char* kind, const std::string& in) {
  Decoder d(in.data(), in.size());
  s2coding::EncodedUintVector<T> vec;
  if (!vec.Init(&d)) {
    Row(kind, HexBytes(in), "reject");
    return;
  }
  std::vector<T> out = vec.Decode();
  Row(kind, HexBytes(in), Ok(in.size() - d.avail(), out.size(), Join(out, UintText<T>)));
}

template <class T>
void Uints(const char* enc_kind, const char* dec_kind) {
  std::vector<std::vector<T>> lists = {{}, {0}, {0, 0, 0}, {std::numeric_limits<T>::max()}};
  for (size_t width = 1; width <= sizeof(T); ++width) {
    for (size_t n : {1, 2, 3, 17}) {
      std::vector<T> v;
      for (size_t i = 0; i < n; ++i) {
        T x = static_cast<T>(rng());
        if (width < sizeof(T)) x &= static_cast<T>((T{1} << (8 * width)) - 1);
        v.push_back(x);
      }
      v[0] |= static_cast<T>(T{1} << (8 * width - 1));  // force exactly this width
      lists.push_back(v);
    }
  }
  for (const auto& v : lists) {
    const std::string bytes = EncodeUints(v);
    Row(enc_kind, Join(v, UintText<T>), HexBytes(bytes));
    for (const std::string& s : WithPrefixes(bytes)) UintDecode<T>(dec_kind, s);
    UintDecode<T>(dec_kind, bytes + "\x07");
  }
  // The K2 header: the largest count the library accepts, with no data (spec K2, FR-019).
  UintDecode<T>(dec_kind, Varint(uint64_t{50000000} * sizeof(T)));
  // A count whose byte size overflows, and the largest varint.
  UintDecode<T>(dec_kind, Varint(std::numeric_limits<uint64_t>::max()));
  UintDecode<T>(dec_kind, Varint(std::numeric_limits<uint64_t>::max() - (sizeof(T) - 1)));
}

// ---- cell-id vector ---------------------------------------------------------------------------

std::string EncodeIds(const std::vector<S2CellId>& v) {
  Encoder e;
  s2coding::EncodeS2CellIdVector(absl::MakeConstSpan(v), &e);
  return Bytes(e);
}

std::string IdText(const S2CellId& c) { return Hex64(c.id()); }

void IdDecode(const std::string& in) {
  Decoder d(in.data(), in.size());
  s2coding::EncodedS2CellIdVector vec;
  if (!vec.Init(&d)) {
    Row("CELLID_VECTOR_DECODE", HexBytes(in), "reject");
    return;
  }
  std::vector<S2CellId> out = vec.Decode();
  Row("CELLID_VECTOR_DECODE", HexBytes(in), Ok(in.size() - d.avail(), out.size(), Join(out, IdText)));
}

void CellIdVectors() {
  std::vector<std::vector<S2CellId>> lists = {{}, {S2CellId(0)}, {S2CellId::None()},
                                              {S2CellId(~uint64_t{0})}};
  // Same-level lists at every level: these reach the odd shifts and the extended code.
  for (int level = 0; level <= S2CellId::kMaxLevel; ++level) {
    for (int n : {1, 2, 5}) {
      std::vector<S2CellId> v;
      for (int i = 0; i < n; ++i) v.push_back(S2CellId(RandUnit()).parent(level));
      lists.push_back(v);
    }
  }
  // Mixed levels, sorted and unsorted, and raw 64-bit patterns with chosen trailing zeros
  // (the format carries any bits, valid cells or not).
  for (int i = 0; i < 20; ++i) {
    std::vector<S2CellId> v;
    int n = 1 + static_cast<int>(rng() % 6);
    for (int k = 0; k < n; ++k) {
      v.push_back(S2CellId(RandUnit()).parent(static_cast<int>(rng() % 31)));
    }
    lists.push_back(v);
  }
  for (int tz = 0; tz < 64; tz += 3) {
    std::vector<S2CellId> v;
    for (int k = 0; k < 3; ++k) v.push_back(S2CellId((rng() | 1) << tz));
    lists.push_back(v);
  }
  for (const auto& v : lists) {
    const std::string bytes = EncodeIds(v);
    Row("CELLID_VECTOR_ENCODE", Join(v, IdText), HexBytes(bytes));
    for (const std::string& s : WithPrefixes(bytes)) IdDecode(s);
    IdDecode(bytes + "\x03");
  }
  // Malformed headers: an extended shift code above 56, every base length with no base,
  // and a valid header followed by the K2 delta header.
  IdDecode(std::string("\xf8\x1c\x00", 3));  // shift_code 31, 29 + 28 = 57 > 56
  IdDecode(std::string("\xf8\x1b\x00", 3));  // 29 + 27 = 56, the largest valid shift
  for (int base_len = 0; base_len < 8; ++base_len) IdDecode(std::string(1, static_cast<char>(base_len)));
  IdDecode(std::string("\x00", 1) + Varint(uint64_t{50000000} * 8));
}

// ---- string vector ------------------------------------------------------------------------------

std::string EncodeStrings(const std::vector<std::string>& v) {
  Encoder e;
  s2coding::StringVectorEncoder::Encode(absl::MakeConstSpan(v), &e);
  return Bytes(e);
}

void StringDecode(const std::string& in) {
  Decoder d(in.data(), in.size());
  s2coding::EncodedStringVector vec;
  if (!vec.Init(&d)) {
    Row("STRING_VECTOR_DECODE", HexBytes(in), "reject");
    return;
  }
  std::vector<absl::string_view> out = vec.Decode();
  std::vector<std::string> items(out.begin(), out.end());
  Row("STRING_VECTOR_DECODE", HexBytes(in),
      Ok(in.size() - d.avail(), items.size(), Join(items, HexBytes)));
}

// A string vector with the given (possibly corrupt) end offsets, followed by `data`.
std::string RawStrings(const std::vector<uint64_t>& offsets, const std::string& data) {
  return EncodeUints(offsets) + data;
}

void StringVectors() {
  std::vector<std::vector<std::string>> lists = {
      {}, {""}, {"", ""}, {"a"}, {"abcde", "", "cdef"}, {std::string(300, 'x')}};
  for (int i = 0; i < 10; ++i) {
    std::vector<std::string> v;
    int n = static_cast<int>(rng() % 5);
    for (int k = 0; k < n; ++k) {
      std::string s;
      int len = static_cast<int>(rng() % 9);
      for (int j = 0; j < len; ++j) s.push_back(static_cast<char>(rng()));
      v.push_back(s);
    }
    lists.push_back(v);
  }
  for (const auto& v : lists) {
    const std::string bytes = EncodeStrings(v);
    // The count disambiguates "no strings" from "one empty string".
    Row("STRING_VECTOR_ENCODE", std::to_string(v.size()) + "|" + Join(v, HexBytes), HexBytes(bytes));
    for (const std::string& s : WithPrefixes(bytes)) StringDecode(s);
    StringDecode(bytes + "\x01");
  }
  // Corrupt offsets (K3, FR-020): decreasing, beyond the data, and a middle entry past the
  // end.
  StringDecode(RawStrings({5, 2, 6}, "abcdef"));
  StringDecode(RawStrings({8, 6}, "abcdef"));
  StringDecode(RawStrings({3, 10}, "abcdef"));
  StringDecode(RawStrings({0, 0, 3}, "abc"));
  // The K2 input: one offset of 499,999,999 with no data.
  StringDecode(RawStrings({499999999}, ""));
}

// ---- point vector ------------------------------------------------------------------------------

std::string EncodePoints(const std::vector<S2Point>& v) {
  Encoder e;
  s2coding::EncodeS2PointVector(absl::MakeConstSpan(v), s2coding::CodingHint::FAST, &e);
  return Bytes(e);
}

void PointDecode(const std::string& in) {
  Decoder d(in.data(), in.size());
  s2coding::EncodedS2PointVector vec;
  if (!vec.Init(&d)) {
    Row("POINT_VECTOR_DECODE", HexBytes(in), "reject");
    return;
  }
  std::vector<S2Point> out = vec.Decode();
  Row("POINT_VECTOR_DECODE", HexBytes(in), Ok(in.size() - d.avail(), out.size(), Join(out, PointText)));
}

S2Point OddPoint(int k) {
  const double nan = std::numeric_limits<double>::quiet_NaN();
  const double inf = std::numeric_limits<double>::infinity();
  switch (k % 4) {
    case 0: return S2Point(nan, 0, 1);
    case 1: return S2Point(inf, -inf, 0);
    case 2: return S2Point(3, 4, 12);  // not unit length
    default: return S2Point(FromBits(0x7ff4000000000001ULL), -0.0, 1e-300);  // signalling NaN bits
  }
}

void PointVectors() {
  std::vector<std::vector<S2Point>> lists = {{}, {RandUnit()}, {RandUnit(), RandUnit(), RandUnit()}};
  for (int k = 0; k < 4; ++k) lists.push_back({OddPoint(k), RandUnit()});
  for (const auto& v : lists) {
    const std::string bytes = EncodePoints(v);
    Row("POINT_VECTOR_ENCODE", Join(v, PointText), HexBytes(bytes));
    for (const std::string& s : WithPrefixes(bytes)) PointDecode(s);
    PointDecode(bytes + "\x02");
  }
  // Unknown format tags 2–7, and the K2 header (50,000,000 points, no data).
  for (int tag = 2; tag < 8; ++tag) PointDecode(std::string(1, static_cast<char>(tag)) + std::string(24, '\0'));
  PointDecode(Varint(uint64_t{50000000} << 3));
}

// ---- fixed-size codecs ---------------------------------------------------------------------------

void CellIdFixed(uint64_t id) {
  Encoder e;
  S2CellId(id).Encode(&e);
  const std::string bytes = Bytes(e);
  Row("CELLID_ENCODE", Hex64(id), HexBytes(bytes));
  for (const std::string& s : WithPrefixes(bytes)) {
    Decoder d(s.data(), s.size());
    S2CellId c;
    if (!c.Decode(&d)) {
      Row("CELLID_DECODE", HexBytes(s), "reject");
    } else {
      Row("CELLID_DECODE", HexBytes(s), Ok(s.size() - d.avail(), 1, Hex64(c.id())),
          c.is_valid() ? "" : "deviation:reject");
    }
  }
}

// C++ has no S2Point::Encode; the lossless point layout (three little-endian doubles) is the
// one EncodedS2PointVector's FAST format and the lossless shape encoders use.
void PointFixed(const S2Point& p) {
  Encoder e;
  e.Ensure(3 * sizeof(double));
  e.putdouble(p.x());
  e.putdouble(p.y());
  e.putdouble(p.z());
  const std::string bytes = Bytes(e);
  Row("POINT_ENCODE", PointText(p), HexBytes(bytes));
  for (const std::string& s : WithPrefixes(bytes)) {
    Decoder d(s.data(), s.size());
    if (d.avail() < 3 * sizeof(double)) {
      Row("POINT_DECODE", HexBytes(s), "reject");
      continue;
    }
    double x = d.getdouble(), y = d.getdouble(), z = d.getdouble();
    Row("POINT_DECODE", HexBytes(s), Ok(s.size() - d.avail(), 1, PointText(S2Point(x, y, z))));
  }
}

std::string CapText(const S2Cap& c) {
  return PointText(c.center()) + ":" + Hex64(Bits(c.radius().length2()));
}

// Spec 007 FR-017: the note for a decoded cap whose length² is negative and not -1.
const char* NegativeNote(const S2Cap& c) {
  const double l = c.radius().length2();
  return (l < 0 && l != -1.0) ? "deviation:negative-canonical" : "";
}

void CapFixed(const S2Cap& cap) {
  Encoder e;
  cap.Encode(&e);
  const std::string bytes = Bytes(e);
  Row("CAP_ENCODE", CapText(cap), HexBytes(bytes));
  for (const std::string& s : WithPrefixes(bytes)) {
    Decoder d(s.data(), s.size());
    S2Cap c;
    if (!c.Decode(&d)) {
      Row("CAP_DECODE", HexBytes(s), "reject");
    } else {
      Row("CAP_DECODE", HexBytes(s), Ok(s.size() - d.avail(), 1, CapText(c)),
          !c.is_valid() ? "deviation:reject" : NegativeNote(c));
    }
  }
}

std::string RectText(const S2LatLngRect& r) {
  return Hex64(Bits(r.lat().lo())) + ":" + Hex64(Bits(r.lat().hi())) + ":" +
         Hex64(Bits(r.lng().lo())) + ":" + Hex64(Bits(r.lng().hi()));
}

void RectDecode(const std::string& s) {
  Decoder d(s.data(), s.size());
  S2LatLngRect r;
  if (!r.Decode(&d)) {
    Row("RECT_DECODE", HexBytes(s), "reject");
  } else {
    Row("RECT_DECODE", HexBytes(s), Ok(s.size() - d.avail(), 1, RectText(r)));
  }
}

void RectFixed(const S2LatLngRect& rect) {
  Encoder e;
  rect.Encode(&e);
  const std::string bytes = Bytes(e);
  Row("RECT_ENCODE", RectText(rect), HexBytes(bytes));
  for (const std::string& s : WithPrefixes(bytes)) RectDecode(s);
  // Version 0 is accepted, version 2 rejected.
  for (char version : {'\0', '\2'}) RectDecode(std::string(1, version) + bytes.substr(1));
}

void CellFixed(S2CellId id) {
  Encoder e;
  S2Cell(id).Encode(&e);
  const std::string bytes = Bytes(e);
  Row("CELL_ENCODE", Hex64(id.id()), HexBytes(bytes));
  for (const std::string& s : WithPrefixes(bytes)) {
    if (s.size() == 8) {
      uint64_t raw;
      std::memcpy(&raw, s.data(), sizeof raw);
      if (!S2CellId(raw).is_valid()) continue;
    }
    Decoder d(s.data(), s.size());
    S2Cell c;
    if (!c.Decode(&d)) {
      Row("CELL_DECODE", HexBytes(s), "reject");
    } else {
      Row("CELL_DECODE", HexBytes(s), Ok(s.size() - d.avail(), 1, Hex64(c.id().id())));
    }
  }
}

void Fixed() {
  for (uint64_t id : {S2CellId::FromFace(3).id(), S2CellId(RandUnit()).id(),
                      S2CellId(RandUnit()).parent(7).id(), uint64_t{0}, uint64_t{7} << 61,
                      uint64_t{1} << 62, ~uint64_t{0}}) {
    CellIdFixed(id);
  }
  PointFixed(RandUnit());
  for (int k = 0; k < 4; ++k) PointFixed(OddPoint(k));
  CapFixed(S2Cap(RandUnit(), S1Angle::Degrees(10)));
  CapFixed(S2Cap::Empty());
  CapFixed(S2Cap::Full());
  CapFixed(S2Cap(S2Point(std::numeric_limits<double>::quiet_NaN(), 0, 1), S1ChordAngle::FromLength2(1)));
  CapFixed(S2Cap(S2Point(3, 4, 12), S1ChordAngle::FromLength2(1)));  // centre not unit length
  // Hand-built cap encodings whose length² is NaN, negative, 4 and above 4. C++ FromLength2 is
  // std::min(4.0, length2), so NaN and anything above 4 decode as a full cap.
  for (uint64_t len2_bits : std::vector<uint64_t>{uint64_t{0x7ff8000000000000}, Bits(-0.5), Bits(4.0), Bits(5.0)}) {
    S2Point c = RandUnit();
    Encoder e;
    e.Ensure(4 * sizeof(double));
    e.putdouble(c.x());
    e.putdouble(c.y());
    e.putdouble(c.z());
    e.putdouble(FromBits(len2_bits));
    const std::string s = Bytes(e);
    Decoder d(s.data(), s.size());
    S2Cap cap;
    if (!cap.Decode(&d)) {
      Row("CAP_DECODE", HexBytes(s), "reject");
    } else {
      // A NaN length² decodes in C++ as a full cap; the library rejects it (D-004, a
      // documented deviation, spec 003 FR-022).
      const bool deviation = !cap.is_valid() || std::isnan(FromBits(len2_bits));
      Row("CAP_DECODE", HexBytes(s), Ok(s.size() - d.avail(), 1, CapText(cap)),
          deviation ? "deviation:reject" : NegativeNote(cap));
    }
  }
  RectFixed(S2LatLngRect(S2LatLng::FromDegrees(-10, 20), S2LatLng::FromDegrees(30, 40)));
  RectFixed(S2LatLngRect::Empty());
  RectFixed(S2LatLngRect::Full());
  RectFixed(S2LatLngRect(S2LatLng::FromDegrees(-10, 170), S2LatLng::FromDegrees(10, -170)));
  // An invalid rectangle (latitude above 90 degrees), encoded by hand.
  {
    Encoder e;
    e.Ensure(1 + 4 * sizeof(double));
    e.put8(1);
    e.putdouble(0.0);
    e.putdouble(2.0);
    e.putdouble(0.0);
    e.putdouble(1.0);
    RectDecode(Bytes(e));
  }
  for (S2CellId id : {S2CellId::FromFace(0), S2CellId(RandUnit()), S2CellId(RandUnit()).parent(12)}) {
    CellFixed(id);
  }
}

}  // namespace

int main() {
  std::printf("kind,input,expected,note\n");
  Varints();
  Uints<uint32_t>("UINT32_ENCODE", "UINT32_DECODE");
  Uints<uint64_t>("UINT64_ENCODE", "UINT64_DECODE");
  CellIdVectors();
  StringVectors();
  PointVectors();
  Fixed();
  return 0;
}
