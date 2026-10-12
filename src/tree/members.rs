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

//! A type as a tree: the trait, the writer of members and their reader.

use super::forms::{hex, key, named, unhex};
use super::{Error, ErrorKind};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde_json::{Map, Value};

/// A type whose values are written as a tree and read from one.
///
/// ```
/// use oxirush_utils::tree::{self, Members, Reading, Refused, Tree};
/// use serde_json::{Value, json};
///
/// #[derive(Debug)]
/// struct Slice {
///     sst: u8,
///     sd: Option<[u8; 3]>,
/// }
///
/// impl Tree for Slice {
///     const NAME: &'static str = "S-NSSAI";
///
///     fn to_tree(&self) -> Value {
///         let mut members = Members::new();
///         members.put("sst", tree::unsigned(self.sst.into()));
///         members.put_optional("sd", self.sd.map(|sd| tree::octets(&sd)));
///         members.finish()
///     }
///
///     fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
///         let members = reading.members(value, &["sst", "sd"])?;
///         Ok(Self {
///             sst: reading.member(&members, 0, |value, reading| reading.unsigned(value, 0, 255))? as u8,
///             sd: reading.optional(&members, 1, |value, reading| reading.octet_array(value))?,
///         })
///     }
/// }
///
/// let slice = Slice { sst: 1, sd: Some([0, 0, 0xff]) };
/// assert_eq!(tree::to_value(&slice), json!({"sst": 1, "sd": "0000ff"}));
///
/// // A name is read however it is written, and a refusal says where and why
/// let read: Slice = tree::from_value(&json!({"SST": 2}))?;
/// assert_eq!((read.sst, read.sd), (2, None));
/// let error = tree::from_value::<Slice>(&json!({"sst": 256})).unwrap_err();
/// assert_eq!(
///     error.to_string(),
///     "this is not a value of S-NSSAI: a number from 0 to 255 is expected, and 256 is written, at sst"
/// );
/// # Ok::<(), oxirush_utils::tree::Error>(())
/// ```
pub trait Tree: Sized {
    /// What the type is called where a value is refused: the name that its specification
    /// gives it.
    const NAME: &'static str;

    /// The tree of the value.
    fn to_tree(&self) -> Value;

    /// The value that `value` writes. What is not a value of the type is refused in
    /// `reading`, which keeps the reason and the names on the way to it.
    fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused>;
}

/// A tree that is not a value of its type: the [`Reading`] has why.
#[derive(Debug)]
pub struct Refused;

/// The tree of `value`.
pub fn to_value<T: Tree>(value: &T) -> Value {
    value.to_tree()
}

/// The value of the type `T` that `value` writes.
///
/// # Errors
///
/// An error of the kind `Value`, or `NoSuchMember` for a name that the type does not
/// have, which says where the tree is not a value of the type, by the names on the way,
/// and why.
pub fn from_value<T: Tree>(value: &Value) -> Result<T, Error> {
    let mut reading = Reading::new();
    match T::from_tree(value, &mut reading) {
        Ok(value) => Ok(value),
        Err(Refused) => Err(reading.refused(T::NAME)),
    }
}

/// The start of what is written, which says which value it is.
fn written(value: &Value) -> String {
    let text = value.to_string();
    match text.char_indices().nth(40) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text,
    }
}

// ── From a value to a tree ──────────────────────────────────────────────────

/// The tree of a value that has members, to which each member that is there is added by
/// its name. The members keep the order in which they are put, which is that of the
/// specification.
#[derive(Debug, Default)]
pub struct Members(Map<String, Value>);

impl Members {
    /// A value without a member yet.
    pub fn new() -> Self {
        Self(Map::new())
    }

    /// A member that is there.
    pub fn put(&mut self, name: &str, value: Value) {
        self.0.insert(name.to_string(), value);
    }

    /// A member that may be absent: nothing is written for one that is.
    pub fn put_optional(&mut self, name: &str, value: Option<Value>) {
        if let Some(value) = value {
            self.put(name, value);
        }
    }

    /// The tree.
    pub fn finish(self) -> Value {
        Value::Object(self.0)
    }
}

// ── From a tree to a value ──────────────────────────────────────────────────

/// What a value that is read is named by, in what holds it.
#[derive(Debug, Clone, Copy)]
enum Name {
    /// A member or an alternative, by its name.
    Member(&'static str),
    /// An entry of a list, by its position.
    Entry(usize),
}

/// The reading of one tree as a value: the names on the way to what is being read, and
/// why the tree is refused, once it is.
#[derive(Debug)]
pub struct Reading {
    at: Vec<Name>,
    /// The first fault: its kind, the names on the way to it, and what is wrong there.
    refusal: Option<(ErrorKind, String, String)>,
}

/// The members of a value that is being read: the object, and where each member that
/// the type has is in it.
#[derive(Debug)]
pub struct Read<'v> {
    names: &'static [&'static str],
    /// The value of each name, in the order of the names: `None` for a member that the
    /// object does not have, or has as `null`.
    values: Vec<Option<&'v Value>>,
}

impl Reading {
    fn new() -> Self {
        Self {
            at: Vec::new(),
            refusal: None,
        }
    }

    /// The error of a value that is refused, for a type that is called `name`.
    fn refused(self, name: &str) -> Error {
        let (kind, at, why) = self.refusal.unwrap_or_else(|| {
            let why = "it is refused without a reason".to_string();
            (ErrorKind::Value, String::new(), why)
        });
        Error::new(kind, at, format!("this is not a value of {name}: {why}"))
    }

    /// Refuse what is being read, for the reason `why`: the first refusal of a reading
    /// is the one that is told.
    pub fn refuse(&mut self, kind: ErrorKind, why: impl Into<String>) -> Refused {
        if self.refusal.is_none() {
            let segment = |name: &Name| match name {
                Name::Member(name) => name.to_string(),
                Name::Entry(position) => position.to_string(),
            };
            let at: Vec<String> = self.at.iter().map(segment).collect();
            self.refusal = Some((kind, at.join("/"), why.into()));
        }
        Refused
    }

    /// Refuse a value that is not written as its type is: `expected` says what is
    /// expected, where `found` is written.
    pub fn mismatch(&mut self, expected: &str, found: &Value) -> Refused {
        let why = format!("{expected} is expected, and {} is written", written(found));
        self.refuse(ErrorKind::Value, why)
    }

    // ── Members, lists and alternatives ─────────────────────────────────────

    /// The members of a value whose type has the members `names`, in their order. A
    /// member is found by its name [however it is written](super::same_name); a member
    /// of another name is refused, and so is a name that is there twice.
    pub fn members<'v>(
        &mut self,
        value: &'v Value,
        names: &'static [&'static str],
    ) -> Result<Read<'v>, Refused> {
        let Some(members) = value.as_object() else {
            return Err(self.mismatch("an object", value));
        };
        let mut values = alloc::vec![None; names.len()];
        let mut found = alloc::vec![false; names.len()];
        for (written, held) in members {
            // The name as its type writes it is the usual case.
            let exact = names.iter().position(|name| *name == written);
            let Some(position) = exact.or_else(|| named(names, written)) else {
                let why = format!(
                    "{written:?} is not a member that its type has: it has {}",
                    names.join(", ")
                );
                return Err(self.refuse(ErrorKind::NoSuchMember, why));
            };
            if found[position] {
                let why = format!("the member {:?} is written twice", names[position]);
                return Err(self.refuse(ErrorKind::Value, why));
            }
            found[position] = true;
            // A member that is null is one that is not there.
            values[position] = Some(held).filter(|held| !held.is_null());
        }
        Ok(Read { names, values })
    }

    /// The member at `position` among the names of a value, which its type always has:
    /// `read` reads it.
    pub fn member<'v, T>(
        &mut self,
        of: &Read<'v>,
        position: usize,
        read: impl FnOnce(&'v Value, &mut Self) -> Result<T, Refused>,
    ) -> Result<T, Refused> {
        self.at.push(Name::Member(of.names[position]));
        let Some(value) = of.values[position] else {
            return Err(self.refuse(ErrorKind::Value, "the member is missing"));
        };
        let value = read(value, self)?;
        self.at.pop();
        Ok(value)
    }

    /// The member at `position` among the names of a value, which may be absent: one
    /// that is left out, or written as `null`.
    pub fn optional<'v, T>(
        &mut self,
        of: &Read<'v>,
        position: usize,
        read: impl FnOnce(&'v Value, &mut Self) -> Result<T, Refused>,
    ) -> Result<Option<T>, Refused> {
        let Some(value) = of.values[position] else {
            return Ok(None);
        };
        self.at.push(Name::Member(of.names[position]));
        let value = read(value, self)?;
        self.at.pop();
        Ok(Some(value))
    }

    /// The entries of a list: `read` reads each.
    pub fn list<'v, T>(
        &mut self,
        value: &'v Value,
        mut read: impl FnMut(&'v Value, &mut Self) -> Result<T, Refused>,
    ) -> Result<Vec<T>, Refused> {
        let Some(entries) = value.as_array() else {
            return Err(self.mismatch("a list", value));
        };
        let mut values = Vec::with_capacity(entries.len());
        for (position, entry) in entries.iter().enumerate() {
            self.at.push(Name::Entry(position));
            values.push(read(entry, self)?);
            self.at.pop();
        }
        Ok(values)
    }

    /// The alternative with content that `value` writes, of a type whose alternatives
    /// are `names`: its position among them, and its content, which the caller reads
    /// [`within`](Self::within) it.
    pub fn alternative<'v>(
        &mut self,
        value: &'v Value,
        names: &'static [&'static str],
    ) -> Result<(usize, &'v Value), Refused> {
        const ONE: &str = "an object with one member, the name of an alternative with its value,";
        let mut members = value.as_object().into_iter().flatten();
        let (Some((written, held)), None) = (members.next(), members.next()) else {
            return Err(self.mismatch(ONE, value));
        };
        let Some(position) = named(names, written) else {
            let why = format!(
                "{written:?} is not an alternative that its type has: it has {}",
                names.join(", ")
            );
            return Err(self.refuse(ErrorKind::NoSuchMember, why));
        };
        Ok((position, held))
    }

    /// Read the content of the alternative, or of the member, that is named `name`.
    pub fn within<'v, T>(
        &mut self,
        name: &'static str,
        value: &'v Value,
        read: impl FnOnce(&'v Value, &mut Self) -> Result<T, Refused>,
    ) -> Result<T, Refused> {
        self.at.push(Name::Member(name));
        let value = read(value, self)?;
        self.at.pop();
        Ok(value)
    }

    // ── Plain values ────────────────────────────────────────────────────────

    /// `true` or `false`.
    pub fn boolean(&mut self, value: &Value) -> Result<bool, Refused> {
        match value.as_bool() {
            Some(read) => Ok(read),
            None => Err(self.mismatch("true or false", value)),
        }
    }

    /// `null`.
    pub fn null(&mut self, value: &Value) -> Result<(), Refused> {
        match value.is_null() {
            true => Ok(()),
            false => Err(self.mismatch("null", value)),
        }
    }

    /// Text.
    pub fn text<'v>(&mut self, value: &'v Value) -> Result<&'v str, Refused> {
        match value.as_str() {
            Some(text) => Ok(text),
            None => Err(self.mismatch("text", value)),
        }
    }

    /// One of the names `names`: its position among them. A name is read [however it is
    /// written](super::same_name); another name is refused with the names that exist.
    pub fn named(&mut self, value: &Value, names: &[&str]) -> Result<usize, Refused> {
        let Some(text) = value.as_str() else {
            return Err(self.mismatch("a name", value));
        };
        named(names, text).ok_or_else(|| {
            let why = format!("{text:?} is none of its names: {}", names.join(", "));
            self.refuse(ErrorKind::Value, why)
        })
    }

    /// A number without a sign from `lower` to `upper`: a JSON number that is whole. A
    /// string of digits and a number with a fraction are refused.
    pub fn unsigned(&mut self, value: &Value, lower: u64, upper: u64) -> Result<u64, Refused> {
        let number = self.number(value, lower.into(), upper.into())?;
        match u64::try_from(number) {
            Ok(number) if (lower..=upper).contains(&number) => Ok(number),
            _ => Err(self.outside(value, lower.into(), upper.into())),
        }
    }

    /// A number with a sign from `lower` to `upper`.
    pub fn signed(&mut self, value: &Value, lower: i64, upper: i64) -> Result<i64, Refused> {
        let number = self.number(value, lower.into(), upper.into())?;
        match i64::try_from(number) {
            Ok(number) if (lower..=upper).contains(&number) => Ok(number),
            _ => Err(self.outside(value, lower.into(), upper.into())),
        }
    }

    /// The whole number that a tree writes: one that 64 bits hold, with a sign or
    /// without.
    fn number(&mut self, value: &Value, lower: i128, upper: i128) -> Result<i128, Refused> {
        let read = value.as_number().and_then(|number| {
            let signed = number.as_i64().map(i128::from);
            signed.or_else(|| number.as_u64().map(i128::from))
        });
        read.ok_or_else(|| self.outside(value, lower, upper))
    }

    fn outside(&mut self, value: &Value, lower: i128, upper: i128) -> Refused {
        self.mismatch(&format!("a number from {lower} to {upper}"), value)
    }

    /// Octets: a string of hexadecimal digits, two for each octet, in either case.
    pub fn octets(&mut self, value: &Value) -> Result<Vec<u8>, Refused> {
        match value.as_str().and_then(unhex) {
            Some(octets) => Ok(octets),
            None => Err(self.mismatch("octets in hexadecimal, two digits for each,", value)),
        }
    }

    /// `N` octets.
    pub fn octet_array<const N: usize>(&mut self, value: &Value) -> Result<[u8; N], Refused> {
        let octets = self.octets(value)?;
        <[u8; N]>::try_from(octets).map_err(|octets| {
            let why = format!(
                "{N} octets are expected, and {} has {}",
                hex(&octets),
                octets.len()
            );
            self.refuse(ErrorKind::Value, why)
        })
    }

    /// A string of bits: its octets and how many bits it has. It is written as its
    /// octets alone, which is then as many bits as they hold, or as [`bits`](super::bits)
    /// writes one whose length its type does not say: `{"value": octets, "length": bits}`.
    /// The length is refused when the octets are more or fewer than it takes, and so is a
    /// bit after the last that is not zero.
    pub fn bits(&mut self, value: &Value) -> Result<(Vec<u8>, usize), Refused> {
        let Some(members) = value.as_object() else {
            let octets = self.octets(value)?;
            let length = octets.len() * 8;
            return Ok((octets, length));
        };
        let _ = members;
        let of = self.members(value, &["value", "length"])?;
        let octets = self.member(&of, 0, |value, reading| reading.octets(value))?;
        let length = self.member(&of, 1, |value, reading| {
            reading.unsigned(value, 0, usize::MAX as u64)
        })? as usize;
        if length.div_ceil(8) != octets.len() {
            let why = format!("{length} bits are not {} octets", octets.len());
            return Err(self.refuse(ErrorKind::Value, why));
        }
        let unused = octets.len() * 8 - length;
        if octets
            .last()
            .is_some_and(|last| last & ((1_u16 << unused) - 1) as u8 != 0)
        {
            let why = format!("the bits after the {length} of the string are not zero");
            return Err(self.refuse(ErrorKind::Value, why));
        }
        Ok((octets, length))
    }
}

/// The member of `members` that `name` names, [however it is written](super::same_name).
pub fn member<'m>(members: &'m Map<String, Value>, name: &str) -> Option<(&'m str, &'m Value)> {
    let known = key(members, name)?;
    Some((known, &members[known]))
}
