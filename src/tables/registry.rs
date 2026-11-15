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

//! The IE types of a protocol, as its table of IE types has them, which `ies!` writes.

use super::{Protocol, Table};

/// The form of an IE type in the table of IE types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// A fixed set of fields of a fixed number of octets.
    Fixed,
    /// A fixed set of fields of a variable number of octets.
    Variable,
    /// Fields that a later release may add to.
    Extendable,
    /// A grouped IE, whose content is IEs.
    Grouped,
}

/// An IE type as the table of IE types of its protocol has it, of the same fields with
/// every feature.
#[derive(Debug)]
pub struct IeTypeInfo<P: Protocol> {
    /// The IE type.
    pub ie_type: u16,
    /// Its name.
    pub name: &'static str,
    /// Its form.
    pub form: Form,
    /// The number of fixed octets that the table gives it.
    pub fixed: Option<u16>,
    /// The clause of its layout, or the table of its content.
    pub clause: &'static str,
    /// The members that the tree of a value may have, as an object: its flags and fields,
    /// and, as the value may have them, `Spare`, `length` and `unknown octets`. A tree has
    /// those of them that the value has, and a path that names another of them selects
    /// nothing. None for a grouped IE, and for a value whose tree is its octets.
    pub members: &'static [&'static str],
    /// The member that the tree of a value is when it is written as a scalar: its one
    /// field, `None` for a value of more, or of none.
    pub scalar: Option<&'static str>,
    /// The tables of its content, for a grouped IE: one, or one per place for an IE that
    /// has different content in different places.
    pub tables: &'static [&'static Table<P>],
}

impl<P: Protocol> Clone for IeTypeInfo<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: Protocol> Copy for IeTypeInfo<P> {}

/// What `ies!` writes of the trees of a protocol's IE types.
#[cfg(feature = "tree")]
pub trait RegistryTree: Protocol {
    /// What the tree of a value of `ie_type` takes: none for a grouped IE.
    fn takes_of(ie_type: u16) -> Option<&'static crate::tree::Takes>;
    /// The tree of the value of `ie_type` that `octets` are, of the content of the table
    /// `table` for a grouped IE, or of its first: `None` for octets that are no such
    /// value, and for a table of another type.
    fn value_tree(
        ie_type: u16,
        table: Option<&'static Table<Self>>,
        octets: &[u8],
    ) -> Option<crate::tree::Value>;
}

#[cfg(feature = "tree")]
impl<P: RegistryTree> IeTypeInfo<P> {
    /// What the tree of a value of the type takes, each of its members with what it takes:
    /// `None` for a grouped IE, whose tree is its IEs by the rows of
    /// [`tables`](Self::tables).
    pub fn takes(&self) -> Option<&'static crate::tree::Takes> {
        P::takes_of(self.ie_type)
    }

    /// The tree of a value of the type from its octets, those after the Length, as the
    /// tree of a message has it: of a grouped IE, with the content of `table`, one of
    /// [`tables`](Self::tables), or of the first of them where `table` is `None`. `None`
    /// for octets that are no value of the type, which a message keeps as octets, and for
    /// a table that is not one of the type.
    pub fn tree_of(
        &self,
        octets: &[u8],
        table: Option<&'static Table<P>>,
    ) -> Option<crate::tree::Value> {
        P::value_tree(self.ie_type, table, octets)
    }
}

/// A message type as the table of message types of its protocol has it.
#[derive(Debug)]
pub struct MessageTypeInfo<P: Protocol> {
    /// The message type.
    pub message_type: u8,
    /// The interfaces that have it.
    pub interfaces: P::Interfaces,
    /// The table of its IEs.
    pub table: &'static Table<P>,
}

impl<P: Protocol> Clone for MessageTypeInfo<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: Protocol> Copy for MessageTypeInfo<P> {}

impl<P: Protocol> MessageTypeInfo<P> {
    /// The name that the table of message types gives it.
    pub fn name(&self) -> &'static str {
        self.table.name
    }
}
