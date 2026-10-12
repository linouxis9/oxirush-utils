#![no_main]
//! See `targets::identity`.

libfuzzer_sys::fuzz_target!(|octets: &[u8]| targets::identity(octets));
