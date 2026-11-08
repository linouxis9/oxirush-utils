#![no_main]
//! See `targets::forms`.

libfuzzer_sys::fuzz_target!(|octets: &[u8]| targets::forms(octets));
