# Contributing to oxirush-utils

## Tests

```sh
cargo test --all-features
cargo test --features alloc
```

The tests of the paths include trees of 65,536 entries, within a time bound.

## Fuzzing

```sh
cargo +nightly fuzz run identity -- -max_total_time=60 -max_len=64
cargo +nightly fuzz run paths -- -max_total_time=60 -max_len=512
cargo +nightly fuzz run forms -- -max_total_time=60
```

`fuzz/` has three [cargo-fuzz](https://crates.io/crates/cargo-fuzz) targets:
`identity` reads octets and text as each identity, `paths` takes any path and
edit on any tree, and `forms` reads any JSON with each reader of a tree. CI
runs each for a minute at a push, an hour each day but Wednesday and Sunday,
and four hours on Wednesday.

## The reader of 3GPP documents

`tools/docx_tables.py` reads the paragraphs, tables and figures of a 3GPP
specification in .docx, for the tests of the protocol crates that compare
their tables and layouts with the document.

## Benchmarks

[`benches/README.md`](benches/README.md) has the figures and the commands.

## Pull requests

A pull request comes with a test for what it changes. Sign off your commits
(`git commit -s`): by contributing you agree to the [Developer Certificate
of Origin](https://developercertificate.org/). [SECURITY.md](SECURITY.md)
says how to report a vulnerability.
