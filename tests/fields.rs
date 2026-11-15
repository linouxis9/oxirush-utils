/*
   OxiRush
   Copyright 2025-2026 Valentin D'Emmanuele

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! The accessors that `bits!` writes over the octets of a value, as a protocol's crate
//! writes them.

#![cfg(feature = "fields")]

extern crate alloc;

use oxirush_utils::fields::Octets;
use oxirush_utils::{bits, named_values};

/// A protocol's row of a field: what `row!` writes.
#[derive(Debug, PartialEq, Eq)]
struct Row {
    ident: &'static str,
    name: &'static str,
    octet: usize,
    low: u8,
    width: u8,
    second: Option<(usize, u8, u8)>,
    values: Option<&'static [(u8, &'static str)]>,
}

macro_rules! row {
    ($field:ident, $name:literal, $octet:literal, $low:literal, $width:tt, [$($octet2:literal $low2:literal $width2:tt)?], [$($kind:ident)?]) => {
        Row {
            ident: stringify!($field),
            name: $name,
            octet: $octet,
            low: $low,
            width: $width,
            second: row!(@second $($octet2 $low2 $width2)?),
            values: row!(@values $($kind)?),
        }
    };
    (@second) => { None };
    (@second $octet:literal $low:literal $width:tt) => { Some(($octet, $low, $width)) };
    (@values) => { None };
    (@values $kind:ident) => { Some(<$kind>::NAMES) };
}

named_values! {
    /// The type of a registration.
    pub enum Kind "Table 1" bits 3 {
        1 Initial "initial registration",
        2 Mobility "mobility registration updating",
    }

}

/// A value of one octet: a flag, a value of a table and a spare bit.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct OneOctet(u8);

bits! {
    over [Row, row]
    impl OneOctet (0) "1.1" octets 1 {
        follow "Follow-on request pending": 0 4 1,
        kind "Registration type": 0 1 3 as Kind,
    }
}

/// A value of octets: a number of two parts and a number in a second octet.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Several(Octets);

bits! {
    over [Row, row]
    impl Several (0) "1.2" octets 3 {
        split "Split": 0 1 4 + 1 5 4,
        count "Count": 2 1 3,
    }
}

#[test]
fn accessors_over_one_octet() {
    let mut value = OneOctet(0b1010);
    assert!(value.follow());
    assert_eq!(value.kind(), Kind::Mobility);
    assert_eq!(value.kind_raw(), 2);
    value.set_follow(false).set_kind(Kind::Initial);
    assert_eq!(value.0, 0b0001);
    value.set_kind_raw(0xff);
    assert_eq!(value.0, 0b0111, "only the bits of the field are written");
    assert_eq!(OneOctet::MASKS, [0b1111]);
    assert!(value.spare_bits_are_zero());
    assert!(!OneOctet(0x80).spare_bits_are_zero());
    assert_eq!(OneOctet::FIELDS.len(), 2);
    assert_eq!(
        OneOctet::FIELDS[1],
        Row {
            ident: "kind",
            name: "Registration type",
            octet: 0,
            low: 1,
            width: 3,
            second: None,
            values: Some(Kind::NAMES)
        }
    );
    assert_eq!(OneOctet::LAYOUT.bits.len(), 2);
}

#[test]
fn accessors_over_octets() {
    // An absent octet reads 0, and a setter adds octets of 0 up to its own
    let mut value = Several(Octets::copied(&[0x0a]));
    assert_eq!(value.split(), 0xa0);
    assert_eq!(value.count(), 0);
    value.set_count(5);
    assert_eq!(&*value.0, [0x0a, 0x00, 0x05]);
    value.set_split(0x3c);
    assert_eq!(&*value.0, [0x03, 0xc0, 0x05]);
    assert_eq!(value.split_raw(), 0x3c);
    assert_eq!(Several::FIELDS[0].second, Some((1, 5, 4)));
    assert_eq!(Several::MASKS, [0x0f, 0xf0, 0x07]);
    assert_eq!(
        Several::LAYOUT.bits.len(),
        3,
        "a field of two parts has a cell for each"
    );
    assert!(!Several(Octets::copied(&[0x10])).spare_bits_are_zero());
}
