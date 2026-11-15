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

//! The fields of the values of a protocol of type, length and value, read in network
//! order: numbers, the narrow values of a few bits, octets after their lengths, lists,
//! addresses, PLMNs, FQDNs, text and digits, and the octets of a key; how a value writes
//! them ([`put`]); what a check finds in them ([`Finding`]), which the protocol makes its
//! deviation ([`Deviate`]); and the layout of each value, which a test compares with the
//! figure of its specification ([`Layout`]).
//!
//! The codecs of the OxiRush crates lay out their values on these fields. A protocol names
//! its own clauses: a finding has the clause of the value that the protocol's macros give
//! it, or none for spare bits, which the protocol's [`Deviate`] gives its clause.

use core::fmt;

mod kinds;
mod octets;

pub use kinds::*;
pub use octets::{Octets, together};

/// The member of a tree that holds the octets after the fields of a value: those of a
/// later release, which the receiver ignores.
pub const UNKNOWN_OCTETS: &str = "unknown octets";

/// A field too long for the bits that say its length, its number or its type, which
/// [`put`] refuses: the IE and the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldLength {
    /// The name of the IE.
    pub ie: &'static str,
    /// The name of the field.
    pub field: &'static str,
}

/// What a check finds in a field or a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finding {
    /// Spare bits that are not zero.
    SpareBits,
    /// A value that its table has as spare, or does not have: the receiver ignores it.
    SpareValue {
        /// The value.
        value: u32,
    },
    /// A value that its table reserves: the receiver takes the IE as incorrect.
    ReservedValue {
        /// The value.
        value: u32,
    },
    /// A value that its table has the sender not send, and the receiver read as another.
    ReadAs {
        /// The value.
        value: u32,
        /// The value that the receiver takes.
        read: u32,
    },
    /// A rule of the clause, in words.
    Rule(&'static str),
}

/// A deviation of a protocol, made of what a check finds.
pub trait Deviate {
    /// The deviation of `finding` at `path`, of `clause`: `None` for spare bits that are
    /// not zero, of the protocol's clause of spare bits.
    fn found(path: &Path<'_>, clause: Option<&'static str>, finding: Finding) -> Self;
}

/// Where an IE is in the tree of its message: written only for a deviation.
#[derive(Debug, Clone, Copy)]
pub enum Path<'a> {
    /// The message level, or a path written already.
    Root(&'a str),
    /// The IE of a row in a scope: a row that is a list has its position.
    Row {
        /// The scope.
        parent: &'a Path<'a>,
        /// The name of the row.
        row: &'static str,
        /// The position in a list.
        nth: Option<usize>,
    },
}

impl fmt::Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root(path) => f.write_str(path),
            Self::Row {
                parent,
                row,
                nth: Some(nth),
            } => write!(f, "{parent}/{}/{nth}", Segment(row)),
            Self::Row {
                parent,
                row,
                nth: None,
            } => write!(f, "{parent}/{}", Segment(row)),
        }
    }
}

/// A name as a segment of a path writes it: `~0` for its `~` and `~1` for its `/`, as a
/// JSON pointer (RFC 6901) and the paths of the trees have them.
pub struct Segment<'a>(pub &'a str);

impl fmt::Display for Segment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rest = self.0;
        while let Some(at) = rest.find(['~', '/']) {
            f.write_str(&rest[..at])?;
            f.write_str(if rest[at..].starts_with('~') {
                "~0"
            } else {
                "~1"
            })?;
            rest = &rest[at + 1..];
        }
        f.write_str(rest)
    }
}

/// The tree of the octets of a name of labels, as a Network Instance or an APN/DNN has
/// them: the name, or, of other octets, an object of one member, the one name of `field`,
/// of the octets in hexadecimal. `{"octets": ...}` is an IE kept as octets.
#[cfg(feature = "tree")]
pub fn name_tree(octets: &[u8], field: &'static [&'static str]) -> crate::tree::Value {
    match fqdn_name(octets) {
        Some(name) => crate::tree::text(name),
        None => crate::tree::alternative(field[0], crate::tree::octets(octets)),
    }
}

/// The octets of the tree of a name of labels ([`name_tree`]).
#[cfg(feature = "tree")]
pub fn name_from_tree(
    value: &crate::tree::Value,
    reading: &mut crate::tree::Reading,
    field: &'static [&'static str],
) -> Result<alloc::vec::Vec<u8>, crate::tree::Refused> {
    if let Some(name) = value.as_str() {
        return fqdn_of(name).ok_or_else(|| {
            reading.mismatch(
                "a name of 253 octets at most, of labels of 1 to 63 octets with dots between them",
                value,
            )
        });
    }
    let (_, octets) = reading.alternative(value, field)?;
    reading.within(field[0], octets, |octets, reading| reading.octets(octets))
}

/// The tree of the octets of a value that is a text, a UTF8String: the text, or, of octets
/// that are no UTF-8, an object of one member, the one name of `field`, of the octets in
/// hexadecimal. `{"octets": ...}` is an IE kept as octets.
#[cfg(feature = "tree")]
pub fn text_tree(octets: &[u8], field: &'static [&'static str]) -> crate::tree::Value {
    match core::str::from_utf8(octets) {
        Ok(text) => crate::tree::text(alloc::string::String::from(text)),
        Err(_) => crate::tree::alternative(field[0], crate::tree::octets(octets)),
    }
}

/// The octets of the tree of a value that is a text ([`text_tree`]), or of a field that
/// is one, whose octets that are no text are the one member `member` of an object. A
/// refusal names no member that is written, which a secret may be.
#[cfg(feature = "tree")]
pub fn text_from_tree(
    value: &crate::tree::Value,
    reading: &mut crate::tree::Reading,
    member: &'static str,
) -> Result<alloc::vec::Vec<u8>, crate::tree::Refused> {
    if let Some(text) = value.as_str() {
        return Ok(text.as_bytes().to_vec());
    }
    let octets = value
        .as_object()
        .filter(|members| members.len() == 1)
        .and_then(|members| crate::tree::member(members, member));
    match octets {
        Some((_, octets)) => {
            reading.within(member, octets, |octets, reading| reading.octets(octets))
        }
        None => {
            let expected = alloc::format!("a text, or its octets as {{\"{member}\": ...}}");
            Err(reading.mismatch(&expected, value))
        }
    }
}
