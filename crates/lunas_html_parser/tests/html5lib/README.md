# Vendored html5lib-tests

The `.test` files under `tokenizer/` are vendored from the
[html5lib-tests](https://github.com/html5lib/html5lib-tests) project, the
standard cross-implementation HTML conformance suite.

- Source: https://github.com/html5lib/html5lib-tests (`tokenizer/` directory)
- License: MIT (see the upstream `LICENSE`)

## How they are used

`tests/html5lib_tokenizer.rs` runs these cases against our lexer. Our tokenizer
is a pragmatic tokenizer for `.lunas` templates, **not** a spec-complete HTML5
tokenizer, so the harness runs the subset within our scope and asserts an exact
match there. Out-of-scope categories (character-reference decoding, DOCTYPE
internals, alternate tokenizer states, NUL/CR normalization, adversarial
mid-tag EOF recovery, etc.) are counted and reported rather than hidden, and a
small explicit list of known per-character recovery divergences is maintained in
the harness as a regression guard.

To refresh the vendored data, re-download the files from the upstream
`tokenizer/` directory.
