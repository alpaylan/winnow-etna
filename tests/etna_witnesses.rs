//! Witness tests for the ETNA workload.
//!
//! Each `witness_*` test calls one of the `property_*` functions in
//! `winnow::etna` with frozen inputs. Tests pass on the base commit and fail
//! when the corresponding mutation is active.

use winnow::etna::{
    property_hex_uint_overflow_errors, property_iterator_no_panic_after_done,
    property_repeat_n_loop_check_errors, PropertyResult,
};

fn assert_pass(r: PropertyResult) {
    match r {
        PropertyResult::Pass => {}
        PropertyResult::Fail(m) => panic!("property failed: {m}"),
        PropertyResult::Discard => panic!("property unexpectedly discarded"),
    }
}

// ---- hex_uint_overflow_b428d65_1 ----

#[test]
fn witness_hex_uint_overflow_case_nine_digits_lowercase() {
    // 9 hex digits — overflows u32 (max 8 nibbles). Base must Err; bug Oks.
    assert_pass(property_hex_uint_overflow_errors(b"123456789".to_vec()));
}

#[test]
fn witness_hex_uint_overflow_case_ten_digits_alpha() {
    assert_pass(property_hex_uint_overflow_errors(b"abcdef0123".to_vec()));
}

#[test]
fn witness_hex_uint_overflow_case_eight_digits_ok() {
    // Exactly 8 — must succeed under both base and bug; serves as a
    // regression check that the property hasn't become too strict.
    assert_pass(property_hex_uint_overflow_errors(b"deadbeef".to_vec()));
}

// ---- iterator_misuse_panic_86b4c25_1 ----

#[test]
fn witness_iterator_misuse_panic_case_short_input() {
    assert_pass(property_iterator_no_panic_after_done(b"ab".to_vec()));
}

#[test]
fn witness_iterator_misuse_panic_case_empty_input() {
    assert_pass(property_iterator_no_panic_after_done(Vec::new()));
}

// ---- repeat_n_loop_check_f5c49ba_1 ----

#[test]
fn witness_repeat_n_loop_check_case_n2() {
    assert_pass(property_repeat_n_loop_check_errors(2));
}

#[test]
fn witness_repeat_n_loop_check_case_n7() {
    assert_pass(property_repeat_n_loop_check_errors(7));
}

