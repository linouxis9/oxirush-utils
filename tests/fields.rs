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

//! The accessors that `bits!` writes over the octets of a value, and the forms of
//! `named_values!`, as a protocol's crate writes them.

#![cfg(feature = "fields")]

extern crate alloc;

use oxirush_utils::fields::{Narrowable, Octets, OutOfRange};
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
    /// The type of a registration, whose other values are read as the first.
    pub enum Kind "Table 1" bits 3 others as 1 {
        1 Initial "initial registration" tree "initial-registration",
        2 Mobility "mobility registration updating" tree "mobility-registration-updating",
    }

    /// A value of two bits, two of whose values are read as another.
    pub enum Level "Table 2" bits 2 read [2 ..= 3 as 1] {
        0 Low "low",
        1 High "high",
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
    value.set_follow(false).set_kind(Kind::Initial).unwrap();
    assert_eq!(value.0, 0b0001);
    // A value of the table that the bits do not hold is refused, and nothing is written
    let refused = value.set_kind(Kind::Other(8)).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "Registration type: 8 is more than 7, the most that its bits hold"
    );
    assert_eq!(value.0, 0b0001);
    value.set_kind(Kind::Other(7)).unwrap();
    assert_eq!(value.0, 0b0111);
    // The setter of bits writes those that the field has
    value.set_kind_raw(0xf9);
    assert_eq!(value.0, 0b0001, "only the bits of the field are written");
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
    value.set_count(5).unwrap();
    assert_eq!(&*value.0, [0x0a, 0x00, 0x05]);
    value.set_split(0x3c).unwrap();
    assert_eq!(&*value.0, [0x03, 0xc0, 0x05]);
    // A number that the bits do not hold is refused, the most that they hold is written
    assert_eq!(
        value.set_count(8).unwrap_err(),
        OutOfRange {
            field: "Count",
            value: 8,
            most: 7
        }
    );
    assert_eq!(value.set_split(0x100).unwrap_err().most, 0xff);
    value.set_split(0xff).unwrap();
    assert_eq!(&*value.0, [0x0f, 0xf0, 0x05]);
    value.set_split(0x3c).unwrap();
    value.set_count_raw(0x0d);
    assert_eq!(
        &*value.0,
        [0x03, 0xc0, 0x05],
        "the bits of the field are written"
    );
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

#[test]
fn named_values_with_tree_names_and_others() {
    assert_eq!(Kind::Initial.name(), Some("initial registration"));
    assert_eq!(Kind::Initial.tree_name(), Some("initial-registration"));
    assert_eq!(Kind::from_value(7).effective(), Kind::Initial);
    assert_eq!(Kind::from_value(2).effective(), Kind::Mobility);
    assert_eq!(Kind::OTHERS_AS, Some(1));
    assert_eq!(Level::from_value(3).effective(), Level::High);
    assert_eq!(Level::OTHERS_AS, None);
    assert_eq!(<Kind as Narrowable>::from_value(2), Kind::Mobility);
    assert_eq!(Kind::named(2), Some(Kind::Mobility));
    assert_eq!(Kind::named(7), None, "a value that the table does not name");
    assert_eq!(Kind::from_value(7), Kind::Other(7));
}

#[cfg(feature = "tree")]
#[test]
fn a_tree_name_is_what_the_tree_writes_and_reads() {
    use oxirush_utils::fields::FieldTree;
    use oxirush_utils::tree;
    assert_eq!(
        FieldTree::to_tree(&Kind::Mobility),
        tree!("mobility-registration-updating")
    );
    let read = tree::read(
        &tree!("Initial Registration"),
        "Kind",
        <Kind as FieldTree>::from_tree,
    );
    assert_eq!(read.ok(), Some(Kind::Initial));
}

/// The macros name what they write by absolute paths: a crate's own `Result` of one
/// parameter, in scope where they expand, changes nothing.
/// A table of the crate alone.
mod private {
    oxirush_utils::named_values! {
        /// A table that its crate keeps to itself.
        pub(crate) enum Hidden "Table 4" bits 1 {
            1 One "one",
        }
    }

    #[test]
    fn a_table_takes_its_visibility() {
        assert_eq!(Hidden::named(1), Some(Hidden::One));
    }
}

mod shadowed {
    #[allow(dead_code)]
    type Result<T> = core::result::Result<T, ()>;

    oxirush_utils::named_values! {
        /// A table of one value.
        pub enum One "Table 3" bits 1 {
            0 Zero "zero",
        }
    }

    #[test]
    fn the_macros_expand_beside_a_result_of_one_parameter() {
        assert_eq!(One::from_value(0).name(), Some("zero"));
    }
}
