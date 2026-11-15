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

//! The tables of a protocol of type, length and value: for a message or a grouped IE, the
//! IEs that it can have, each with its presence, how many times it may be there and the
//! interfaces that have it; and what a typed scope does with them, written once for every
//! table of every protocol.
//!
//! A protocol says what is its own through [`Protocol`]: the key of an IE in its scope
//! (its type, or its type and its instance), its framing, the IEs that a scope keeps as
//! octets, its presences and its interfaces.

use alloc::vec::Vec;
use core::fmt::Debug;

mod macros;
#[cfg(feature = "tree")]
mod paths;
mod registry;

#[cfg(feature = "tree")]
pub use paths::{
    Of, Paths, RowAt, TreeProtocol, TreeScope, field_tree, located, read_field, row_at,
    scope_from_tree, scope_to_tree, select, set,
};

#[cfg(feature = "tree")]
pub use registry::RegistryTree;
pub use registry::{Form, IeTypeInfo, MessageTypeInfo};

/// What a protocol gives the tables: the key of an IE, its framing, its kept IEs, its
/// presences and its interfaces.
pub trait Protocol: Debug + Sized + 'static {
    /// What identifies an IE in its scope: its type, or its type and its instance.
    type Key: Copy + Eq + Debug;
    /// An IE that a scope keeps as octets.
    type Raw: Raw<Key = Self::Key, Error = Self::Error>;
    /// Why a message or an IE does not encode.
    type Error;
    /// Why the IEs of a scope do not frame.
    type Fault;
    /// The presence of a row in its table.
    type Presence: Presence;
    /// The interfaces of a row: one, or a set of them.
    type Interfaces: Interfaces;

    /// The IEs of `octets`, each its key and its content, until one that does not frame.
    fn ies(octets: &[u8]) -> impl Iterator<Item = Result<(Self::Key, &[u8]), Self::Fault>>;
    /// Whether a scope reads an IE of `key` and `content` as the value of a row: not one
    /// that it keeps as octets whatever its table, as one of a null length.
    fn placeable(key: Self::Key, content: &[u8]) -> bool;
    /// The IE that a scope keeps as octets, of `key` and `content`.
    fn raw(key: Self::Key, content: &[u8]) -> Self::Raw;
}

/// An IE that a scope keeps as octets.
pub trait Raw {
    /// Its key.
    type Key;
    /// Why it does not encode.
    type Error;
    /// Its key.
    fn key(&self) -> Self::Key;
    /// Write it.
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), Self::Error>;
    /// How many octets it writes, its header included.
    fn encoded_len(&self) -> usize;
}

/// The presence of an IE in its table.
pub trait Presence: Copy + Eq + Debug {
    /// Whether the sender always includes it.
    fn is_mandatory(self) -> bool;
    /// Its rank from the strongest: the weakest of several is the one of the highest rank.
    fn rank(self) -> u8;
}

/// The interfaces of a row: one, or a set of them.
pub trait Interfaces: Copy + Eq + Debug {
    /// No interface.
    const NONE: Self;
    /// Whether it has one of `other`.
    fn has(self, other: Self) -> bool;
    /// Whether it has each of `other`.
    fn covers(self, other: Self) -> bool;
    /// It with `other`.
    fn union(self, other: Self) -> Self;
    /// Each interface of the set, one by one.
    fn each(self) -> impl Iterator<Item = Self>;
}

/// How many IEs of a row a scope may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    /// One: the receiver handles the first and ignores the others.
    Once,
    /// As many as there are: "several IEs with the same IE type may be present".
    Any,
    /// At most this many.
    AtMost(u8),
}

impl Repeat {
    /// Whether `count` IEs may be there.
    pub const fn allows(self, count: usize) -> bool {
        match self {
            Self::Once => count <= 1,
            Self::Any => true,
            Self::AtMost(most) => count <= most as usize,
        }
    }
}

/// One row of a table: an IE that the scope can have, a field of the scope. A field that
/// several rows of one key share, which the wire does not tell apart, is one row of
/// those roles.
#[derive(Debug)]
pub struct Row<P: Protocol> {
    /// The name that the table gives the IE. A field that several rows share has the name
    /// of the IE type, and `roles` the names of the rows.
    pub name: &'static str,
    /// The key of its IEs: the IE type, or the IE type and its instance.
    pub ie_type: P::Key,
    /// The rows of the table that the field is, each with its presence, its interfaces
    /// and how many IEs of it may be there: the row itself, or each row that shares the
    /// field. [`on`](Self::on) gives what they are on an interface.
    pub roles: &'static [Role<P>],
    /// The table of the content of a grouped IE in this place, for an IE type whose
    /// content differs by place. `None` for another IE.
    pub content: Option<&'static Table<P>>,
    /// Whether the check of the scope evaluates a condition of this row from the other IEs
    /// of the message.
    pub checked: bool,
}

/// A row of a table, as the field of a scope has it: the one of a field of its own, or one
/// of the rows that share a field, which the wire does not tell apart.
#[derive(Debug)]
pub struct Role<P: Protocol> {
    /// The name that the table gives the row.
    pub name: &'static str,
    /// Its presence.
    pub presence: P::Presence,
    /// How many IEs of it may be there.
    pub repeat: Repeat,
    /// The interfaces that have it.
    pub interfaces: P::Interfaces,
}

/// A table: of a message, or of the content of a grouped IE in one place.
#[derive(Debug)]
pub struct Table<P: Protocol> {
    /// Its number in the specification.
    pub id: &'static str,
    /// The name of the message or of the IE.
    pub name: &'static str,
    /// The message type of a message, or the IE type of a grouped IE.
    pub code: u16,
    /// Its rows, in the order of the table.
    pub rows: &'static [Row<P>],
    /// The interfaces that have a constructor of their own, for a table whose mandatory
    /// IEs differ between them: none for a table of one `new()`.
    pub constructors: P::Interfaces,
}

/// What a row is on an interface, or on a set of them, as [`Row::on`] gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct On<P: Protocol> {
    /// Its presence: the weakest of its roles there; on a set of interfaces, the weakest on
    /// each, and `None` where one of them lacks the row, which a message on the set may
    /// have and needs on none, whatever the condition beside it.
    pub presence: Option<P::Presence>,
    /// How many IEs of it a message needs: one for each of its mandatory roles there.
    pub needed: usize,
    /// How many IEs of it a message may have: as many as its roles there allow, `Once` for
    /// one.
    pub most: Repeat,
}

/// How many IEs `roles` allow together, `usize::MAX` for any number: what the field of a
/// row holds, which the table macro asserts.
#[allow(dead_code)]
pub const fn capacity<P: Protocol>(roles: &[Role<P>]) -> usize {
    let mut at = 0;
    let mut most = 0usize;
    while at < roles.len() {
        most = most.saturating_add(match roles[at].repeat {
            Repeat::Once => 1,
            Repeat::AtMost(at_most) => at_most as usize,
            Repeat::Any => usize::MAX,
        });
        at += 1;
    }
    most
}

impl<P: Protocol> Row<P> {
    /// What the row is on `interfaces`: `None` where none of them has it, as its IEs are
    /// discarded there. On a set of interfaces, a message may have the IEs of the roles of
    /// any of them, and needs what each of them needs: the weakest presence of each, none
    /// where one of them lacks the row, and the fewest IEs that one of them needs.
    pub fn on(&self, interfaces: P::Interfaces) -> Option<On<P>> {
        // A row of one role, as most are: its presence where each of the interfaces has it
        match self.roles {
            [role] if role.interfaces.has(interfaces) => {
                let each = role.interfaces.covers(interfaces);
                Some(On {
                    presence: each.then_some(role.presence),
                    needed: usize::from(each && role.presence.is_mandatory()),
                    most: match role.repeat {
                        Repeat::AtMost(1) => Repeat::Once,
                        repeat => repeat,
                    },
                })
            }
            roles => on_roles(roles, interfaces),
        }
    }

    /// Whether the row is a list in the tree of its scope: of a role that may be there more
    /// than once, or of roles that share the field.
    pub fn is_list(&self) -> bool {
        match self.roles {
            [role] => role.repeat != Repeat::Once,
            _ => true,
        }
    }
}

/// What rows of `roles` are on `interfaces`, as [`Row::on`] says.
#[doc(hidden)]
pub fn on_roles<P: Protocol>(roles: &[Role<P>], interfaces: P::Interfaces) -> Option<On<P>> {
    if !roles.iter().any(|role| role.interfaces.has(interfaces)) {
        return None;
    }
    let rank = |presence: &P::Presence| presence.rank();
    // Of each interface: the weakest presence of its roles, and the IEs of its mandatory
    // roles.
    let each = interfaces.each().map(|one| {
        let there = roles.iter().filter(move |role| role.interfaces.has(one));
        let presence = there.clone().map(|role| role.presence).max_by_key(rank);
        let needed = there.filter(|role| role.presence.is_mandatory());
        (presence, needed.count())
    });
    let (presence, needed) = each
        .reduce(|(was, before), (presence, needed)| {
            let weakest = was
                .zip(presence)
                .map(|(was, presence)| core::cmp::max_by_key(was, presence, rank));
            (weakest, before.min(needed))
        })
        .unwrap_or_default();
    let most = roles
        .iter()
        .filter(|role| role.interfaces.has(interfaces))
        .try_fold(0usize, |most, role| match role.repeat {
            Repeat::Once => Some(most + 1),
            Repeat::AtMost(at) => Some(most + usize::from(at)),
            Repeat::Any => None,
        });
    let most = match most.map(u8::try_from) {
        Some(Ok(1)) => Repeat::Once,
        Some(Ok(most)) => Repeat::AtMost(most),
        _ => Repeat::Any,
    };
    Some(On {
        presence,
        needed,
        most,
    })
}

impl<P: Protocol> Table<P> {
    /// The interfaces that its rows have.
    pub fn interfaces(&self) -> P::Interfaces {
        let roles = self.rows.iter().flat_map(|row| row.roles.iter());
        roles.fold(P::Interfaces::NONE, |all, role| all.union(role.interfaces))
    }

    /// The row of `ie_type`.
    pub fn row_of(&self, ie_type: P::Key) -> Option<&'static Row<P>> {
        let rows: &'static [Row<P>] = self.rows;
        rows.iter().find(|row| row.ie_type == ie_type)
    }
}

/// The values of a field of a scope: one, in its place or boxed, or a list. The
/// functions that each table writes for its rows call one generic function per kind of
/// field and type of value, which all the tables share.
pub trait FieldValues<T> {
    /// The values, in their order.
    fn values(&self) -> &[T];
    /// The first value: that of a field that is there once.
    #[inline(always)]
    fn first(&self) -> Option<&T> {
        self.values().first()
    }
    /// Set the value, or add it to the list.
    fn add(&mut self, value: T);
}

impl<T> FieldValues<T> for Option<T> {
    #[inline(always)]
    fn values(&self) -> &[T] {
        self.as_slice()
    }
    #[inline]
    fn add(&mut self, value: T) {
        *self = Some(value);
    }
}

impl<T> FieldValues<T> for Option<alloc::boxed::Box<T>> {
    #[inline(always)]
    fn values(&self) -> &[T] {
        match self {
            Some(value) => core::slice::from_ref(&**value),
            None => &[],
        }
    }
    #[inline]
    fn add(&mut self, value: T) {
        *self = Some(alloc::boxed::Box::new(value));
    }
}

impl<T> FieldValues<T> for Vec<T> {
    #[inline(always)]
    fn values(&self) -> &[T] {
        self
    }
    #[inline]
    fn add(&mut self, value: T) {
        self.push(value);
    }
}

/// An entry of the `order` of a scope, the order of the wire, which a decoded message keeps
/// when its IEs are not in the order of their tables, so that it encodes back byte for
/// byte: the next value of a field, or the next IE of the scope's `raw`. A scope without
/// an order is encoded in the order of its table, each IE of its `raw` after the values of
/// its row and those of no row last; a value added to a scope of an order goes in the
/// place of its row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed<K> {
    /// The next value of the field of this key.
    Field(K),
    /// The next IE of the scope's raw IEs.
    Raw,
}

/// What a typed scope did with an IE of its octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placing {
    /// It is the value of its field.
    Placed,
    /// Its row is there as many times as the table allows already.
    Repeated,
    /// Its value does not decode as the type of its row.
    Malformed,
    /// The table has no row of its key.
    Unexpected,
}

/// What the table macro writes for a message or a grouped IE, which the code of every
/// scope takes: the fields by key, the raw IEs and the order.
pub trait Scope: Default {
    /// Its protocol.
    type P: Protocol;
    /// The table of the scope.
    fn table() -> &'static Table<Self::P>;
    /// The position in the table of the row of `ie_type`.
    fn position(ie_type: <Self::P as Protocol>::Key) -> Option<usize>;
    /// Whether a list of the scope is of values large enough to be sized before it is
    /// filled, by a first pass over the IEs.
    const PRESIZE: bool;
    /// Size the lists for the IEs of `octets`.
    fn presize(&mut self, octets: &[u8]);
    /// Read the content of an IE of `ie_type` into its field.
    fn place(&mut self, ie_type: <Self::P as Protocol>::Key, body: &[u8]) -> Placing;
    /// Write the `nth` value of the field of `ie_type`: `false` for one that is not there.
    fn emit(
        &self,
        ie_type: <Self::P as Protocol>::Key,
        nth: usize,
        out: &mut Vec<u8>,
    ) -> Result<bool, <Self::P as Protocol>::Error>;
    /// How many octets the IEs of the scope have, their headers included.
    fn encoded_len(&self) -> usize;
    /// Write every field, in the order of the table.
    fn encode_fields(&self, out: &mut Vec<u8>) -> Result<(), <Self::P as Protocol>::Error>;
    /// The octets that the scope takes beyond its own size: its lists and boxes, and the
    /// IEs that it keeps as octets. 0 where its crate does not count them.
    fn held(&self) -> usize {
        0
    }
    /// How many values the field of `ie_type` has.
    fn count(&self, ie_type: <Self::P as Protocol>::Key) -> usize;
    /// Remove the values of the field of `ie_type`.
    fn clear(&mut self, ie_type: <Self::P as Protocol>::Key);
    /// Whether `octets` decode as the value of the field of `ie_type`.
    fn decodes(&self, ie_type: <Self::P as Protocol>::Key, octets: &[u8]) -> bool;
    /// Whether its octets read as the scope: it keeps no IE as octets, has no more values
    /// of a row than the row takes, and each value writes octets that read as it, here
    /// and in each grouped IE that it holds. A null length reads as no value, and a raw IE
    /// as what its octets are.
    fn plain(&self) -> bool;
    /// Whether it has no IE: its octets are none.
    fn is_empty(&self) -> bool;
    /// The IEs that the scope keeps as octets.
    fn raw(&self) -> &[<Self::P as Protocol>::Raw];
    /// The same, to change.
    fn raw_mut(&mut self) -> &mut Vec<<Self::P as Protocol>::Raw>;
    /// The order of the wire, when it is not that of the table.
    fn order(&self) -> &[Placed<<Self::P as Protocol>::Key>];
    /// The same, to change.
    fn order_mut(&mut self) -> &mut Vec<Placed<<Self::P as Protocol>::Key>>;
}

/// The key of an IE of a scope's protocol.
type Key<S> = <<S as Scope>::P as Protocol>::Key;
/// Why a scope's protocol does not encode.
type Error<S> = <<S as Scope>::P as Protocol>::Error;
/// Why a scope's IEs do not frame.
type Fault<S> = <<S as Scope>::P as Protocol>::Fault;

/// The scope that its octets read: `None` for one that does not encode, or whose octets
/// do not decode.
pub fn reread<S: Scope>(scope: &S) -> Option<S> {
    let mut octets = Vec::new();
    encode_scope(scope, &mut octets).ok()?;
    decode_scope(&octets).ok()
}

/// Decode a grouped IE in a box that `slot`, which is empty, takes: `false`, and the slot
/// left empty, for octets that are not one.
pub fn decode_boxed<S: Scope>(slot: &mut Option<alloc::boxed::Box<S>>, octets: &[u8]) -> bool {
    let scope = slot.insert(alloc::boxed::Box::default());
    if decode_scope_into(&mut **scope, octets).is_err() {
        *slot = None;
        return false;
    }
    true
}

/// The scope of `octets`.
pub fn decode_scope<S: Scope>(octets: &[u8]) -> Result<S, Fault<S>> {
    let mut scope = S::default();
    decode_scope_into(&mut scope, octets)?;
    Ok(scope)
}

/// The scope of `octets`, in `scope`, which is empty.
pub fn decode_scope_into<S: Scope>(scope: &mut S, octets: &[u8]) -> Result<(), Fault<S>> {
    decode_scope_sized(scope, octets, false)
}

/// The scope of `octets`, in `scope`, which is empty; its large lists sized first by a
/// pass over its IEs where `presize` asks for it: a message, whose lists of grouped IEs
/// are long, and not a grouped IE, whose lists are short or empty.
pub fn decode_scope_sized<S: Scope>(
    scope: &mut S,
    octets: &[u8],
    presize: bool,
) -> Result<(), Fault<S>> {
    if presizes::<S>(presize) {
        scope.presize(octets);
    }
    // No order is kept while the IEs come in the order of the table, which encoding the
    // scope without one gives back.
    let mut order: Option<Vec<Placed<Key<S>>>> = None;
    let mut last = (0, false);
    // The rows there once whose first IE the scope keeps as octets, of a null length or
    // not decoding: the receiver handles that first one, and ignores the others.
    let mut kept_first: Vec<Key<S>> = Vec::new();
    for ie in <S::P as Protocol>::ies(octets) {
        let (ie_type, body) = ie?;
        let placing = match !<S::P as Protocol>::placeable(ie_type, body) {
            true => Placing::Unexpected,
            false if kept_first.contains(&ie_type) => Placing::Repeated,
            false => scope.place(ie_type, body),
        };
        // No row is of a type that a scope never places, which a null length leaves out.
        if unread(placing, body) && once::<S>(ie_type) && !kept_first.contains(&ie_type) {
            kept_first.push(ie_type);
        }
        let placed = match placing {
            Placing::Placed => Placed::Field(ie_type),
            _ => {
                scope.raw_mut().push(<S::P as Protocol>::raw(ie_type, body));
                Placed::Raw
            }
        };
        if let Some(order) = &mut order {
            order.push(placed);
            continue;
        }
        let at = rank::<S>(ie_type, placed == Placed::Raw);
        if at >= last {
            last = at;
            continue;
        }
        // The first IE out of the order of the table: those before it were in that order.
        let raws = scope.raw();
        let before = match placed {
            Placed::Raw => &raws[..raws.len() - 1],
            Placed::Field(_) => raws,
        };
        let count = |ie_type| scope.count(ie_type) - usize::from(placed == Placed::Field(ie_type));
        let mut kept = in_table_order(S::table(), before, count);
        kept.push(placed);
        order = Some(kept);
    }
    *scope.order_mut() = order.unwrap_or_default();
    Ok(())
}

/// Whether a scope sizes its lists first: a message whose lists are of large values.
fn presizes<S: Scope>(presize: bool) -> bool {
    presize && S::PRESIZE
}

/// Whether an IE of `body` that a scope placed so is kept as octets unread: of a null
/// length, or that does not decode as its row's value.
fn unread(placing: Placing, body: &[u8]) -> bool {
    placing == Placing::Malformed || body.is_empty()
}

/// Whether the row of `ie_type` in a scope is there once.
fn once<S: Scope>(ie_type: Key<S>) -> bool {
    S::table().row_of(ie_type).is_some_and(|row| !row.is_list())
}

/// Where an IE goes in the order of a table: at the position of its row, an IE that the
/// scope keeps as octets after the values of its row, and one of no row after every row.
fn rank<S: Scope>(ie_type: Key<S>, kept: bool) -> (usize, bool) {
    (S::position(ie_type).unwrap_or(usize::MAX), kept)
}

/// The order of a scope that has `count` values of each row and keeps `raws`, in the order
/// of its table: each row, its values then its kept IEs, and the kept IEs of no row last.
fn in_table_order<P: Protocol>(
    table: &Table<P>,
    raws: &[P::Raw],
    count: impl Fn(P::Key) -> usize,
) -> Vec<Placed<P::Key>> {
    let kept = |ie_type: P::Key| raws.iter().filter(|ie| ie.key() == ie_type).count();
    let mut order = Vec::new();
    for row in table.rows {
        order.extend(core::iter::repeat_n(
            Placed::Field(row.ie_type),
            count(row.ie_type),
        ));
        order.extend(core::iter::repeat_n(Placed::Raw, kept(row.ie_type)));
    }
    let of_no_row = raws.iter().filter(|ie| table.row_of(ie.key()).is_none());
    order.extend(core::iter::repeat_n(Placed::Raw, of_no_row.count()));
    order
}

/// Keep `order` in `scope`, unless it is the order of the table, which the scope without
/// one is encoded in: then nothing is kept, and a scope that is changed stays in that order.
pub fn keep_order<S: Scope>(scope: &mut S, order: Vec<Placed<Key<S>>>) {
    let ordered = in_table_order(S::table(), scope.raw(), |ie_type| scope.count(ie_type));
    *scope.order_mut() = if order == ordered { Vec::new() } else { order };
}

/// An entry of a row in the tree of its scope: a value of its field, or an IE that the
/// scope keeps as octets, by its position among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// The value of this position in the field.
    Value(usize),
    /// The IE of this position in the scope's raw IEs.
    Kept(usize),
}

/// The entries of the row of `ie_type`, as the tree of the scope lists them: in the order
/// of the wire as far as the scope keeps one, then the values, then the kept IEs.
pub fn each_entry<S: Scope>(scope: &S, ie_type: Key<S>, mut each: impl FnMut(Entry)) {
    let (raws, count) = (scope.raw(), scope.count(ie_type));
    let (mut values, mut kept) = (0, 0);
    for placed in scope.order() {
        match *placed {
            Placed::Field(of) if of == ie_type && values < count => {
                each(Entry::Value(values));
                values += 1;
            }
            Placed::Raw => {
                if raws.get(kept).is_some_and(|ie| ie.key() == ie_type) {
                    each(Entry::Kept(kept));
                }
                kept += 1;
            }
            Placed::Field(_) => {}
        }
    }
    (values..count).for_each(|nth| each(Entry::Value(nth)));
    for (at, ie) in raws.iter().enumerate().skip(kept) {
        if ie.key() == ie_type {
            each(Entry::Kept(at));
        }
    }
}

/// The position in the tree of `scope` of its value `nth` of `ie_type`: an IE kept as
/// octets has a position of its own before or between the values.
pub fn place<S: Scope>(scope: &S, ie_type: Key<S>, nth: usize) -> usize {
    let kept = scope.raw().iter().any(|ie| ie.key() == ie_type);
    match S::table().row_of(ie_type).filter(|_| kept) {
        Some(row) => value_positions(scope, row).get(nth).copied().unwrap_or(nth),
        None => nth,
    }
}

/// The position of each value of `row` in the list of the row in the tree of the scope,
/// which keeps IEs of the row as octets: the tree lists the values and those IEs.
pub fn value_positions<S: Scope>(scope: &S, row: &Row<S::P>) -> Vec<usize> {
    let mut positions = Vec::new();
    positions_of(scope, row, |entry, position| {
        if let Entry::Value(_) = entry {
            positions.push(position);
        }
    });
    positions
}

/// The same of each IE that the scope keeps as octets, by its position in `raw`: `None`
/// for one of no row too.
pub fn kept_positions<S: Scope>(scope: &S) -> Vec<Option<usize>> {
    let raws = scope.raw();
    let mut positions = alloc::vec![None; raws.len()];
    let kept = |row: &&Row<S::P>| raws.iter().any(|ie| ie.key() == row.ie_type);
    for row in S::table().rows.iter().filter(kept) {
        let mut of_row = Vec::new();
        let entries = positions_of(scope, row, |entry, position| {
            if let Entry::Kept(at) = entry {
                of_row.push((at, position));
            }
        });
        let listed = row.is_list() || entries > 1;
        for (at, position) in of_row {
            positions[at] = listed.then_some(position);
        }
    }
    positions
}

/// Each entry of `row` with its position in the tree, and how many there are.
fn positions_of<S: Scope>(scope: &S, row: &Row<S::P>, mut each: impl FnMut(Entry, usize)) -> usize {
    let mut entries = 0;
    each_entry(scope, row.ie_type, |entry| {
        each(entry, entries);
        entries += 1;
    });
    entries
}

/// More than the rows of a table, which the table macro asserts: the counts of each row
/// that the encoder keeps on the stack.
pub const MAX_ROWS: usize = 64;

/// Write `scope`: in the order that it keeps, with the values that were added since in
/// the places of their rows, or in the order of its table, where an IE that it keeps as
/// octets follows the values of its row, and one of no row comes last.
pub fn encode_scope<S: Scope>(scope: &S, out: &mut Vec<u8>) -> Result<(), Error<S>> {
    let order = scope.order();
    let table = S::table();
    if order.is_empty() {
        let raws = scope.raw();
        let of_row = |ie: &&<S::P as Protocol>::Raw| table.row_of(ie.key()).is_some();
        match raws.iter().any(|ie| of_row(&ie)) {
            false => scope.encode_fields(out)?,
            true => {
                for row in table.rows {
                    let mut nth = 0;
                    while scope.emit(row.ie_type, nth, out)? {
                        nth += 1;
                    }
                    for ie in raws.iter().filter(|ie| ie.key() == row.ie_type) {
                        ie.encode(out)?;
                    }
                }
            }
        }
        for ie in raws.iter().filter(|ie| !of_row(ie)) {
            ie.encode(out)?;
        }
        return Ok(());
    }
    // How many values of each row the order has: those after are added.
    let mut ordered = [0usize; MAX_ROWS];
    for placed in order {
        if let Placed::Field(ie_type) = placed
            && let Some(at) = S::position(*ie_type)
        {
            ordered[at] += 1;
        }
    }
    let mut next_added = 0;
    let mut emit_added = |before: usize, out: &mut Vec<u8>| -> Result<(), Error<S>> {
        while next_added < table.rows.len() && next_added < before {
            let ie_type = table.rows[next_added].ie_type;
            let mut nth = ordered[next_added];
            while scope.emit(ie_type, nth, out)? {
                nth += 1;
            }
            next_added += 1;
        }
        Ok(())
    };
    let mut emitted = [0usize; MAX_ROWS];
    let mut raw = scope.raw().iter();
    for placed in order {
        match placed {
            Placed::Field(ie_type) => {
                let at = S::position(*ie_type).unwrap_or(table.rows.len());
                emit_added(at, out)?;
                scope.emit(*ie_type, emitted[at], out)?;
                emitted[at] += 1;
            }
            Placed::Raw => {
                if let Some(ie) = raw.next() {
                    ie.encode(out)?;
                }
            }
        }
    }
    emit_added(usize::MAX, out)?;
    for ie in raw {
        ie.encode(out)?;
    }
    Ok(())
}
