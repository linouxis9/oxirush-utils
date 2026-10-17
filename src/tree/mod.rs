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

//! One tree for the messages of every family: the tree of a value as JSON, the forms of
//! its plain values, and paths into it.
//!
//! A tree is a `serde_json::Value`. A type gives its tree and is read from one through
//! [`Tree`], with [`Members`] to write its members and [`Reading`] to read them; a
//! refusal says the member and why. [`select`], [`paths`], [`set`], [`remove`] and
//! [`insert`] take a path into a tree.
//!
//! ```
//! use oxirush_utils::tree;
//! use serde_json::json;
//!
//! let mut message = json!({
//!     "message-type": "registration-request",
//!     "requested-nssai": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}],
//! });
//!
//! // Each slice, by a path that names its members however they are written
//! assert_eq!(tree::select(&message, "/Requested NSSAI/*/sst")?, [&json!(1), &json!(2)]);
//!
//! // An edit is done whole, or not at all
//! tree::set(&mut message, "/requested-nssai/0/sd", json!("000001"))?;
//! tree::remove(&mut message, "/requested-nssai/1")?;
//! println!("{message}");
//! # Ok::<(), tree::Error>(())
//! ```
//!
//! ```text
//! {"message-type":"registration-request","requested-nssai":[{"sst":1,"sd":"000001"}]}
//! ```
//!
//! # The forms
//!
//! | Value | Form |
//! |-------|------|
//! | Octets | A string of hexadecimal digits, written in lower case and read in either: `"02f839"`. No prefix, no separator |
//! | A whole number | A JSON number, exact from -2^63 to 2^64 - 1. A string of digits and a number with a fraction are refused |
//! | A named value | Its name, read [however it is written](same_name) |
//! | One of several alternatives | Its name, or `{"name": content}` for one that has content |
//! | A string of bits | Its octets, or `{"value": octets, "length": bits}` where its type does not say its length |
//! | A value with members | An object, with its members in the order of the specification |
//! | A member that is absent | Left out. `null` is read as absent, and selects nothing |
//!
//! # The order of the members
//!
//! The members of an object keep the order in which they were written, which is that of
//! the specification. The feature `tree` turns on `serde_json/preserve_order` for that,
//! so the order does not depend on what else a build enables. Cargo unifies features:
//! in a program that depends on this crate with `tree`, every `serde_json::Map` keeps
//! the order of insertion, those of the program too.
//!
//! # Paths
//!
//! A path is the segments of a JSON pointer (RFC 6901): `/` before each, `~1` for a `/`
//! and `~0` for a `~` in a segment. A segment is the name of a member, the position of
//! an entry of a list as decimal writes it, `*` for each member or each entry, or, as
//! the last segment of an [`insert`], `-` for the end of a list. The empty path is the
//! tree itself. A path has 4096 octets and 64 segments at most, and selects 4096 values
//! at most.
//!
//! A family of messages whose trees are more than objects and lists says so through
//! [`Family`]: its root, an entry that a name selects, a member that is absent and
//! could be there, a value that it shows opened ([`open_in`]). The functions that end
//! with `_in` take one.
//!
//! An edit is done in the place of the tree, whole or not at all: what it changes is
//! kept and put back when a later place cannot take it, and nothing of the tree is
//! copied, so that an edit of one value of a tree of 65,535 entries takes what the
//! walk to it takes.

mod error;
mod forms;
mod members;
mod path;
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub mod serde;
mod walk;

pub use error::{Error, ErrorKind};
pub use forms::{
    Form, alternative, bits, hex, named, octets, same_name, signed, text, unhex, unsigned,
    written_as,
};
pub use members::{Members, Read, Reading, Refused, Tree, from_value, member, to_value};
pub use path::{MAX_OCTETS, MAX_SEGMENTS, MAX_SELECTED, escape, position, segments};
pub use walk::{
    Absent, Edit, Family, Key, Listed, Next, Place, Plain, Removal, Root, Step, insert, insert_in,
    open_in, paths, paths_in, places_in, remove, remove_in, select, select_forms_in, select_in,
    set, set_in,
};
