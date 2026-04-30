# winnow — Injected Bugs

Parser-combinator library for Rust — ETNA workload mining the winnow git history.

Total mutations: 3

## Bug Index

| # | Variant | Name | Location | Injection | Fix Commit |
|---|---------|------|----------|-----------|------------|
| 1 | `hex_uint_overflow_b428d65_1` | `hex_uint_overflow` | `src/ascii/mod.rs:1298` | `marauders` | `b428d6504dd5f600ebe2a13bdd7b6510aa7d4a17` |
| 2 | `iterator_misuse_panic_86b4c25_1` | `iterator_misuse_panic` | `src/combinator/multi.rs:50` | `patch` | `86b4c25b33aadc3095333b31afd89be6cc9c6e82` |
| 3 | `repeat_n_loop_check_f5c49ba_1` | `repeat_n_loop_check` | `src/combinator/multi.rs:714` | `marauders` | `f5c49ba6607517c7eea73a32ffef66515f4b7049` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `hex_uint_overflow_b428d65_1` | `HexUintOverflowErrors` | `witness_hex_uint_overflow_case_nine_digits_lowercase`, `witness_hex_uint_overflow_case_ten_digits_alpha`, `witness_hex_uint_overflow_case_eight_digits_ok` |
| `iterator_misuse_panic_86b4c25_1` | `IteratorNoPanicAfterDone` | `witness_iterator_misuse_panic_case_short_input`, `witness_iterator_misuse_panic_case_empty_input` |
| `repeat_n_loop_check_f5c49ba_1` | `RepeatNLoopCheckErrors` | `witness_repeat_n_loop_check_case_n2`, `witness_repeat_n_loop_check_case_n7` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `HexUintOverflowErrors` | ✓ | ✓ | ✓ | ✓ |
| `IteratorNoPanicAfterDone` | ✓ | ✓ | ✓ | ✓ |
| `RepeatNLoopCheckErrors` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. hex_uint_overflow

- **Variant**: `hex_uint_overflow_b428d65_1`
- **Location**: `src/ascii/mod.rs:1298` (inside `ascii::hex_uint`)
- **Property**: `HexUintOverflowErrors`
- **Witness(es)**:
  - `witness_hex_uint_overflow_case_nine_digits_lowercase`
  - `witness_hex_uint_overflow_case_ten_digits_alpha`
  - `witness_hex_uint_overflow_case_eight_digits_ok`
- **Source**: fix(char): Error on hex_uint overflow
  > `hex_uint::<u32>` (then named `hex_u32`) silently truncated inputs longer than the type could hold by taking `min(invalid_offset, max_offset)`. The fix returns an `Err` whenever the input has more leading hex digits than the type's `max_nibbles`, instead of dropping the leading nibbles and parsing the trailing 8.
- **Fix commit**: `b428d6504dd5f600ebe2a13bdd7b6510aa7d4a17` — fix(char): Error on hex_uint overflow
- **Invariant violated**: `winnow::ascii::hex_uint::<_, u32, _>(s)` must return `Err` when `s` consists entirely of hex digits and has length strictly greater than `max_nibbles::<u32>() = 8`. Conversely it must return `Ok` for hex-digit-only inputs of length 1..=8.
- **How the mutation triggers**: The buggy branch reads `Ok(max_offset) => invalid_offset.min(max_offset)`. For a 10-byte all-hex input, `invalid_offset = 10` and `max_offset = 8`, so the parser silently consumes 8 nibbles and reports success with the trailing 2 bytes as remainder, dropping the leading nibbles. The fix replaces the `min` with an explicit comparison that errors when `max_offset < invalid_offset`.

### 2. iterator_misuse_panic

- **Variant**: `iterator_misuse_panic_86b4c25_1`
- **Location**: `src/combinator/multi.rs:50` (inside `combinator::iterator`)
- **Property**: `IteratorNoPanicAfterDone`
- **Witness(es)**:
  - `witness_iterator_misuse_panic_case_short_input`
  - `witness_iterator_misuse_panic_case_empty_input`
- **Source**: fix(comb): Prevent misuse of 'iterator' from panicking
  > `combinator::iterator()` returns a `ParserIterator` whose state machine was held in `Option<State<E>>` and consumed via `.take().unwrap()`. Once the iterator transitioned to `Done`/`Cut`, a follow-up `next()` call would `unwrap()` a `None` and panic. The fix replaces the `Option` with `State<E>` directly and uses `matches!` so post-completion `next()` simply returns `None`.
- **Fix commit**: `86b4c25b33aadc3095333b31afd89be6cc9c6e82` — fix(comb): Prevent misuse of 'iterator' from panicking
- **Invariant violated**: Calling `next()` on a `ParserIterator` after it has returned `None` (because the embedded parser backtracked or cut) must continue to return `None` and never panic. Calling `finish()` on an exhausted iterator must succeed without panicking.
- **How the mutation triggers**: The patch reinstates the buggy state representation: `state: Option<State<E>>` plus `let State::Running = self.state.take().unwrap()` inside `next()` and `match self.state.take().unwrap() { ... }` inside `finish()`. After the first `None`, the state field is left as `None`; subsequent calls into `next()`/`finish()` execute `.unwrap()` on `None` and panic with `called Option::unwrap() on a None value`.

### 3. repeat_n_loop_check

- **Variant**: `repeat_n_loop_check_f5c49ba_1`
- **Location**: `src/combinator/multi.rs:714` (inside `combinator::fold_repeat_n_`)
- **Property**: `RepeatNLoopCheckErrors`
- **Witness(es)**:
  - `witness_repeat_n_loop_check_case_n2`
  - `witness_repeat_n_loop_check_case_n7`
- **Source**: [#365](https://github.com/winnow-rs/winnow/issues/365) — fix(comb)!: Add missing infinite loop check to repeat
  > The count-bounded `repeat(n, parser)` path (`fold_repeat_n_`) had no progress check: if `parser` succeeded without consuming any input, `repeat(n, parser)` silently accumulated n outputs over the same position. The fix adds an `eof_offset` comparison that produces a `ParserError::assert` (debug-build panic, release-build error) when the inner parser doesn't make progress, mirroring the unbounded `repeat0/repeat1` paths.
- **Fix commit**: `f5c49ba6607517c7eea73a32ffef66515f4b7049` — fix(comb)!: Add missing infinite loop check to repeat
- **Invariant violated**: For any `n >= 1` and any parser `p` that succeeds without consuming input, `repeat(n, p).parse_next(&mut "")` must not return `Ok` — either an assert panic (debug) or `Err(ParserError::assert(..))` (release) is the correct outcome.
- **How the mutation triggers**: The buggy variant deletes the `if input.eof_offset() == len { return Err(ParserError::assert(...)); }` block. With `space0` as the inner parser, every call succeeds matching zero whitespace and consumes no input; the loop runs `n` times and returns `Ok(vec![""; n])`.

## Dropped Candidates

- `9b31e4f` (fix(stream): Don't overflow stack on PartialEq/PartialOrd) — Bug triggers infinite recursion in BStr/Bytes ↔ &[u8]/str equality, causing process-killing stack overflow on every platform; cannot be observed via property_*+catch_unwind because the OS aborts the runner before any failure signal reaches the adapter.
