#![no_main]
//! See `targets::paths`.

libfuzzer_sys::fuzz_target!(|octets: &[u8]| targets::paths(octets));
