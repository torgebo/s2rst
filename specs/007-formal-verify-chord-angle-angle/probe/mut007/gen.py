# Generates spec 007's mutant patches (M-6NN) from exact-string edits of the current sources.
import difflib, os, sys
ROOT = "/home/t/sources/s2rst_multi/3"
CA, AG = "core/src/s1/chord_angle.rs", "core/src/s1/angle.rs"
CS, AS = "verify/src/chord_spec.rs", "verify/src/angle_spec.rs"
M = [
 ("601","from-length2-keeps-negatives","P-CAVAL-03, P-CAVAL-02","ca_val::p_caval_03 ca_val::p_caval_02__ctor","the FR-017 fix removed: `from_length2` keeps negatives above −1e300 (option B undone)",CA,"        if length2 < 0.0 {\n            ChordAngle::NEGATIVE","        if length2 < -1.0e300 {\n            ChordAngle::NEGATIVE"),
 ("602","from-length2-keeps-nan","P-CAVAL-03, P-CAVAL-02","ca_val::p_caval_03 ca_val::p_caval_02__ctor","`from_length2` keeps NaN (FR-018 undone)",CA,"        } else if length2 <= MAX_LENGTH2 {\n            ChordAngle(length2)","        } else if !(length2 > MAX_LENGTH2) {\n            ChordAngle(length2)"),
 ("603","from-length2-clamps-to-right","P-CAVAL-03","ca_val::p_caval_03","`from_length2` clamps above 4 to `RIGHT`",CA,"            ChordAngle(length2)\n        } else {\n            ChordAngle::STRAIGHT","            ChordAngle(length2)\n        } else {\n            ChordAngle::RIGHT"),
 ("604","from-angle-no-negative-case","P-CAVAL-04","ca_val::p_caval_04","`from_angle` without its negative case: a negative angle gives a length",CA,"        if angle.radians() < 0.0 {","        if angle.radians() < -1.0e300 {"),
 ("605","is-special-without-infinity","P-CACLS-01","ca_cls::p_cacls_01","`is_special` misses `INFINITY`",CA,"        self.is_negative() || self.is_infinity()","        self.is_negative()"),
 ("606","successor-of-straight-steps","P-CANBR-01, P-CAVAL-02","ca_nbr::p_canbr_01 ca_val::p_caval_02__nbr","`successor` of `STRAIGHT` steps one ulp past 4 instead of to `INFINITY`",CA,"        if self.0 >= MAX_LENGTH2 {\n            ChordAngle::INFINITY","        if self.0 > MAX_LENGTH2 {\n            ChordAngle::INFINITY"),
 ("607","predecessor-of-tiny-is-negative","P-CANBR-02","ca_nbr::p_canbr_02","`predecessor` of the smallest length skips `+0`",CA,"        if self.0 <= 0.0 {\n            ChordAngle::NEGATIVE","        if self.0 <= f64::from_bits(1) {\n            ChordAngle::NEGATIVE"),
 ("608","predecessor-steps-up","P-CANBR-02, P-CANBR-03","ca_nbr::p_canbr_02 ca_nbr::p_canbr_03","`predecessor` steps up",CA,"            ChordAngle(f64_next_after(self.0, -10.0))","            ChordAngle(f64_next_after(self.0, 10.0))"),
 ("609","plus-error-moves-specials","P-CAERR-01","ca_err::p_caerr_01","`plus_error` without the special check",CA,"        if self.is_special() {\n            return self;\n        }\n        // C++'s std::min","        // C++'s std::min"),
 ("610","plus-error-below-zero","P-CAERR-01, P-CAVAL-02","ca_err::p_caerr_01 ca_val::p_caval_02__err","`plus_error` clamps below at −0.5, not 0",CA,"        ChordAngle(if 0.0 < at_most_4 { at_most_4 } else { 0.0 })","        ChordAngle(if 0.0 < at_most_4 { at_most_4 } else { -0.5 })"),
 ("611","max-point-error-4","P-CAERR-02","ca_err::p_caerr_02__point","`max_point_error` with 4·ε for 4.5·ε",CA,"        4.5 * f64::EPSILON * self.0","        4.0 * f64::EPSILON * self.0"),
 ("612","add-without-straight","P-CAARI-01","ca_ari::p_caari_01","`Add` without its `STRAIGHT` branch",CA,"        if a2 + b2 >= MAX_LENGTH2 {\n            return ChordAngle::STRAIGHT;\n        }\n","        "),
 ("613","add-zero-returns-rhs","P-CAARI-01","ca_ari::p_caari_01","`Add` returns the zero operand for a zero `rhs`",CA,"            return self;\n        }\n        if a2 + b2 >= MAX_LENGTH2 {","            return rhs;\n        }\n        if a2 + b2 >= MAX_LENGTH2 {"),
 ("614","sub-larger-returns-rhs","P-CAARI-02","ca_ari::p_caari_02","`Sub` returns `rhs` where it should return `ZERO`",CA,"        if a2 <= b2 {\n            return ChordAngle::ZERO;","        if a2 <= b2 {\n            return rhs;"),
 ("615","sin2-half","P-CAARI-03","ca_ari::p_caari_03__sin2","`sin2` with 0.5 for 0.25",CA,"        self.0 * (1.0 - 0.25 * self.0)","        self.0 * (1.0 - 0.5 * self.0)"),
 ("616","cos-plus","P-CAARI-03","ca_ari::p_caari_03__cos","`cos` adds",CA,"        1.0 - 0.5 * self.0","        1.0 + 0.5 * self.0"),
 ("617","right-is-one","P-CAVAL-01","ca_val::p_caval_01","`RIGHT` as 1",CA,"pub const RIGHT: ChordAngle = ChordAngle(2.0);","pub const RIGHT: ChordAngle = ChordAngle(1.0);"),
 ("618","spec-less-or-equal","P-CAORD-01","ca_ord::p_caord_01","description: `chord_spec::less` is `<=`",CS,"    a < b\n}","    a <= b\n}"),
 ("619","spec-next-skips-zero","P-CANBR-01","ca_nbr::p_canbr_01","description: `chord_spec::next` of −1 skips +0",CS,"    if l == NEGATIVE {\n        ZERO","    if l == NEGATIVE {\n        TINY"),
 ("620","from-angle-past-4","P-CAVAL-02, P-CAVAL-04","ca_val::p_caval_02__angle ca_val::p_caval_04","`from_angle`'s length plus one, past 4",CA,"            ChordAngle(l * l)","            ChordAngle(l * l + 1.0)"),
 ("621","add-without-clamp","P-CAVAL-02, P-CAARI-01","ca_val::p_caval_02__ari ca_ari::p_caari_01","`Add` without its clamp at 4",CA,"        ChordAngle((x + y + 2.0 * (x * y).sqrt()).min(MAX_LENGTH2))","        ChordAngle(x + y + 2.0 * (x * y).sqrt())"),
 ("631","from-radians-negates","P-AGVAL-01","ag_val::p_agval_01","`from_radians` negates",AG,"        Angle(radians)\n","        Angle(-radians)\n"),
 ("632","abs-identity","P-AGVAL-01","ag_val::p_agval_01","`abs` is the identity",AG,"        Angle(self.0.abs())","        Angle(self.0)"),
 ("633","neg-identity","P-AGVAL-01","ag_val::p_agval_01","negation is the identity",AG,"        Angle(-self.0)","        Angle(self.0)"),
 ("634","from-degrees-inverted","P-AGCNV-01, P-AGCNV-02","ag_cnv::p_agcnv_02__degrees","`from_degrees` with 180/π",AG,"        Angle(degrees * (PI / 180.0))","        Angle(degrees * (180.0 / PI))"),
 ("635","degrees-inverted","P-AGCNV-01, P-AGCNV-02","ag_cnv::p_agcnv_02__degrees","`degrees` with π/180",AG,"        self.0 * (180.0 / PI)","        self.0 * (PI / 180.0)"),
 ("636","from-e6-scale-e7","P-AGCNV-01","","`from_e6` with 1e-7",AG,"        Self::from_degrees(f64::from(e6) * 1e-6)","        Self::from_degrees(f64::from(e6) * 1e-7)"),
 ("637","from-unsigned-e7-as-e6","P-AGCNV-01","","`from_unsigned_e7` goes through `from_e6`",AG,"        Self::from_e7(e7 as i32)","        Self::from_e6(e7 as i32)"),
 ("638","e7-ties-away","P-AGEF-01","ag_eform::p_agef_01__e7","`e7` rounds ties away from zero (FR-022 undone)",AG,"        (self.degrees() * 1e7).round_ties_even() as i32","        (self.degrees() * 1e7).round() as i32"),
 ("639","e5-scale-1e6","P-AGEF-01","ag_eform::p_agef_01__e5","`e5` with 1e6",AG,"        (self.degrees() * 1e5).round_ties_even() as i32","        (self.degrees() * 1e6).round_ties_even() as i32"),
 ("640","e6-truncates","P-AGEF-01","ag_eform::p_agef_01__e6","`e6` truncates",AG,"        (self.degrees() * 1e6).round_ties_even() as i32","        (self.degrees() * 1e6) as i32"),
 ("641","add-subtracts","P-AGARI-01","ag_ari::p_agari_01__add","`Add` subtracts",AG,"        Angle(self.0 + rhs.0)","        Angle(self.0 - rhs.0)"),
 ("642","scalar-mul-divides","P-AGARI-01","ag_ari::p_agari_01__mul","`f64 * Angle` divides",AG,"        Angle(self * rhs.0)","        Angle(rhs.0 / self)"),
 ("643","ratio-reversed","P-AGARI-01","ag_ari::p_agari_01__ratio","`Angle / Angle` reversed",AG,"        self.0 / rhs.0","        rhs.0 / self.0"),
 ("644","from-e7-perturbed","P-AGCNV-01","","`from_e7`'s scale two ulps off",AG,"        Self::from_degrees(f64::from(e7) * 1e-7)","        Self::from_degrees(f64::from(e7) * 1.0000000000000002e-7)"),
 ("645","spec-round-away","P-AGEF-01","ag_eform::p_agef_01__e6","description: `angle_spec::round_half_even` rounds ties up",AS,"    if d < 0.5 || (d == 0.5 && even) {","    if d < 0.5 {"),
 ("646","normalized-pi-goes-negative","P-AGNRM-01","","`normalized` subtracts 2π at π, giving −π",AG,"        if r > PI {\n            Angle(r - y)","        if r >= PI {\n            Angle(r - y)"),
 ("647","normalized-old-formula","P-AGNRM-01","","the FR-021 fix removed: `normalized` reduces with the old inexact formula (K7)",AG,"        let r = self.0 % y;","        let r = self.0 - (self.0 / y).round() * y;"),
 ("648","e7-truncates","P-AGEF-02","","`e7` truncates instead of rounding",AG,"        (self.degrees() * 1e7).round_ties_even() as i32","        (self.degrees() * 1e7).trunc() as i32"),
]
SLOW = {"615", "638", "639", "640", "642", "643", "645"}
ENUM = "verify --release --test angle_enumeration -- --ignored"
TESTS = {
    "636": [f"{ENUM} p_agcnv_01_e_forms"],
    "637": [f"{ENUM} p_agcnv_01_unsigned"],
    "644": [f"{ENUM} p_agcnv_01_e_forms"],
    "648": [f"{ENUM} p_agef_02_e7"],
    "646": ["core --lib s1::angle::angle_tests"],
    "647": ["core --lib s1::angle::angle_tests"],
}
out = os.path.join(ROOT, "verify/mutants")
for (n, slug, targets, expect, defect, path, old, new) in M:
    src = open(os.path.join(ROOT, path)).read()
    assert src.count(old) == 1, (n, src.count(old), old[:60])
    dst = src.replace(old, new)
    diff = difflib.unified_diff(src.splitlines(True), dst.splitlines(True), f"a/{path}", f"b/{path}", n=3)
    body = "".join(diff)
    body = body.replace(f"--- a/{path}\n", f"diff --git a/{path} b/{path}\n--- a/{path}\n", 1)
    tier = "Tier: slow\n" if n in SLOW else ""
    tests = "".join(f"Test: {t}\n" for t in TESTS.get(n, []))
    head = f"Mutant: M-{n}\nTargets: {targets}\nExpect-Fail: {expect}\n{tier}{tests}Defect: {defect}\n\n"
    open(os.path.join(out, f"M-{n}-{slug}.patch"), "w").write(head + body)
print(len(M), "patches")
