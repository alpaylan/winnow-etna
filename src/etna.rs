//! ETNA benchmark harness.
//!
//! Defines the framework-neutral `PropertyResult` enum plus one
//! `property_*` function per mined bug. Every framework adapter in
//! `src/bin/etna.rs` and every witness test calls into these functions.

#![allow(missing_docs)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

// ---------------------------------------------------------------------------
// hex_uint must err on overflow, not silently truncate.
// (commit b428d65 — "fix(char): Error on hex_uint overflow")
// ---------------------------------------------------------------------------

/// `hex_uint::<_, u32, _>(input)` must NOT silently truncate when the input
/// contains more than 8 leading hex digits. The fix at b428d65 makes it
/// return an error in that case; the bug returned a partial value plus a
/// trailing-bytes remainder, dropping leading nibbles entirely.
///
/// The property: take any non-empty hex-only `&[u8]` slice. If the buggy
/// behaviour keeps reading 8 bytes regardless of length, then for inputs
/// longer than 8 hex digits, the parser would still succeed and consume only
/// 8 bytes (leaving the rest unread). The fixed parser must err on those.
pub fn property_hex_uint_overflow_errors(input: Vec<u8>) -> PropertyResult {
    use crate::ascii::hex_uint;
    use crate::error::InputError;
    use crate::Parser;

    if input.is_empty() {
        return PropertyResult::Discard;
    }
    // Restrict the property to inputs that are entirely valid hex digits;
    // those are the only inputs where the bug surfaces. Anything else makes
    // both base and buggy code stop at the first non-hex char.
    if !input
        .iter()
        .all(|c| c.is_ascii_hexdigit())
    {
        return PropertyResult::Discard;
    }
    let mut s: &[u8] = &input;
    let res: Result<u32, InputError<&[u8]>> = hex_uint.parse_next(&mut s);
    if input.len() > 8 {
        match res {
            Err(_) => PropertyResult::Pass,
            Ok(v) => PropertyResult::Fail(format!(
                "hex_uint::<u32> on {}-byte hex input {:?} returned Ok({:#x}) (truncation), expected Err",
                input.len(),
                String::from_utf8_lossy(&input),
                v
            )),
        }
    } else {
        // Inputs <= 8 hex digits must parse successfully.
        match res {
            Ok(_) => PropertyResult::Pass,
            Err(e) => PropertyResult::Fail(format!(
                "hex_uint::<u32> on {}-byte hex input {:?} returned Err({:?}), expected Ok",
                input.len(),
                String::from_utf8_lossy(&input),
                e
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// `iterator()` combinator must not panic when iterated past completion.
// (commit 86b4c25 — "fix(comb): Prevent misuse of 'iterator' from panicking")
// ---------------------------------------------------------------------------

/// `iterator()` returns a `ParserIterator`. Driving the iterator via `next()`
/// past its terminal state (Done / Cut) must not panic — even if a caller
/// then asks for `finish()`. The bug stored state in `Option<State>` and
/// called `.take().unwrap()` on every `next()`, panicking after the first
/// `None`.
///
/// The property: for any input bytes plus a max-iter cap, the call sequence
/// "next() repeatedly until None, then next() one more time, then finish()"
/// must complete without panicking.
pub fn property_iterator_no_panic_after_done(input: Vec<u8>) -> PropertyResult {
    use crate::combinator::iterator;
    use crate::error::InputError;
    use crate::token::any;
    use crate::Parser;

    let max = input.len();
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let mut bytes: &[u8] = &input;
        fn parse_one<'a>(
            i: &mut &'a [u8],
        ) -> Result<u8, InputError<&'a [u8]>> {
            any.parse_next(i)
        }
        let mut it = iterator(&mut bytes, parse_one);
        let mut consumed: usize = 0;
        loop {
            match (&mut it).next() {
                Some(_) => {
                    consumed += 1;
                    if consumed > max + 4 {
                        break;
                    }
                }
                None => break,
            }
        }
        // Already exhausted. Exercise the post-exhaustion path that used to
        // panic: call next() again (multiple times), then finish().
        let _ = (&mut it).next();
        let _ = (&mut it).next();
        let _ = it.finish();
        consumed
    }));
    match res {
        Ok(_) => PropertyResult::Pass,
        Err(_) => PropertyResult::Fail(format!("iterator panicked after exhaustion")),
    }
}

// ---------------------------------------------------------------------------
// `repeat(n, p)` must error/assert when `p` accepts without consuming.
// (commit f5c49ba — "fix(comb)!: Add missing infinite loop check to repeat")
// ---------------------------------------------------------------------------

/// `repeat(n, p)` with `p` that succeeds without consuming any input must
/// fail (the fix uses `ParserError::assert`). Without the consume-check,
/// `repeat(n, p)` would silently accumulate `n` copies of nothing, masking
/// a misuse that with unbounded `repeat(.., p)` would have been an infinite
/// loop. The property: feed `repeat(N, ascii::space0)` an empty input — the
/// space0 parser matches 0+ whitespace successfully without consuming, so a
/// fixed parser must err and a buggy parser must succeed with `N` items.
pub fn property_repeat_n_loop_check_errors(n: u8) -> PropertyResult {
    use crate::ascii::space0;
    use crate::combinator::repeat;
    use crate::error::InputError;
    use crate::Parser;

    if n < 1 {
        return PropertyResult::Discard;
    }
    // Cap n so we don't blow the test runner.
    let n = (n as usize) % 16 + 1;
    // The fix uses `ParserError::assert(...)`; with `debug_assertions` enabled
    // that panics rather than returning a normal Err. Either path counts as
    // "fixed behaviour"; only an Ok result indicates the buggy parser.
    let panicked = std::panic::catch_unwind(|| {
        let mut input: &str = "";
        let res: Result<Vec<&str>, InputError<&str>> =
            repeat(n, space0).parse_next(&mut input);
        res
    });
    match panicked {
        Err(_) => PropertyResult::Pass,
        Ok(Err(_)) => PropertyResult::Pass,
        Ok(Ok(v)) => PropertyResult::Fail(format!(
            "repeat({}, space0) on \"\" returned Ok with {} elements; expected an assert error or panic",
            n,
            v.len()
        )),
    }
}

