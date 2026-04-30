# winnow — ETNA Tasks

Total tasks: 12

## Task Index

| Task | Variant | Framework | Property | Witness |
|------|---------|-----------|----------|---------|
| 001 | `hex_uint_overflow_b428d65_1` | proptest | `HexUintOverflowErrors` | `witness_hex_uint_overflow_case_nine_digits_lowercase` |
| 002 | `hex_uint_overflow_b428d65_1` | quickcheck | `HexUintOverflowErrors` | `witness_hex_uint_overflow_case_nine_digits_lowercase` |
| 003 | `hex_uint_overflow_b428d65_1` | crabcheck | `HexUintOverflowErrors` | `witness_hex_uint_overflow_case_nine_digits_lowercase` |
| 004 | `hex_uint_overflow_b428d65_1` | hegel | `HexUintOverflowErrors` | `witness_hex_uint_overflow_case_nine_digits_lowercase` |
| 005 | `iterator_misuse_panic_86b4c25_1` | proptest | `IteratorNoPanicAfterDone` | `witness_iterator_misuse_panic_case_short_input` |
| 006 | `iterator_misuse_panic_86b4c25_1` | quickcheck | `IteratorNoPanicAfterDone` | `witness_iterator_misuse_panic_case_short_input` |
| 007 | `iterator_misuse_panic_86b4c25_1` | crabcheck | `IteratorNoPanicAfterDone` | `witness_iterator_misuse_panic_case_short_input` |
| 008 | `iterator_misuse_panic_86b4c25_1` | hegel | `IteratorNoPanicAfterDone` | `witness_iterator_misuse_panic_case_short_input` |
| 009 | `repeat_n_loop_check_f5c49ba_1` | proptest | `RepeatNLoopCheckErrors` | `witness_repeat_n_loop_check_case_n2` |
| 010 | `repeat_n_loop_check_f5c49ba_1` | quickcheck | `RepeatNLoopCheckErrors` | `witness_repeat_n_loop_check_case_n2` |
| 011 | `repeat_n_loop_check_f5c49ba_1` | crabcheck | `RepeatNLoopCheckErrors` | `witness_repeat_n_loop_check_case_n2` |
| 012 | `repeat_n_loop_check_f5c49ba_1` | hegel | `RepeatNLoopCheckErrors` | `witness_repeat_n_loop_check_case_n2` |

## Witness Catalog

- `witness_hex_uint_overflow_case_nine_digits_lowercase` — base passes, variant fails
- `witness_hex_uint_overflow_case_ten_digits_alpha` — base passes, variant fails
- `witness_hex_uint_overflow_case_eight_digits_ok` — base passes, variant fails
- `witness_iterator_misuse_panic_case_short_input` — base passes, variant fails
- `witness_iterator_misuse_panic_case_empty_input` — base passes, variant fails
- `witness_repeat_n_loop_check_case_n2` — base passes, variant fails
- `witness_repeat_n_loop_check_case_n7` — base passes, variant fails
