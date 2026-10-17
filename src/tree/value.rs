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

//! The value of a tree: JSON whose objects keep their members in the order in which they
//! are written, whatever the features of `serde_json` in a build.

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::ops;

pub use serde_json::Number;

/// A tree: a JSON value whose objects keep their members in the order in which they were
/// written, which is that of the specification for the tree of a message.
///
/// It has the variants and the calls of `serde_json::Value`, and converts to and from
/// one. Two objects are equal when they have the same members, whatever their order, as
/// two JSON objects are. It prints as JSON, its members in their order, and `{:#}`
/// prints it indented.
///
/// ```
/// use oxirush_utils::tree::{Map, Value};
///
/// let mut slice = Map::new();
/// slice.insert("sst".into(), 1.into());
/// slice.insert("sd".into(), "0000ff".into());
/// let tree = Value::Object(slice);
/// assert_eq!(tree.to_string(), r#"{"sst":1,"sd":"0000ff"}"#);
/// assert_eq!(tree, serde_json::json!({"sd": "0000ff", "sst": 1}));
/// assert_eq!(tree["sst"], 1);
/// ```
#[derive(Clone, Default)]
pub enum Value {
    /// `null`.
    #[default]
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number.
    Number(Number),
    /// A string.
    String(String),
    /// A list.
    Array(Vec<Value>),
    /// An object, its members in the order in which they were written.
    Object(Map),
}

/// The members of an object of a tree, in the order in which they were written: a member
/// that is added goes at the end, one that is given another value keeps its place, and
/// one that is taken out leaves the others in theirs.
///
/// It is a list. An object of more than 32 members also keeps the position of each name
/// in an index, a copy of its names in their order, so that a member is found by its name
/// in a time that grows as log n: whatever builds an object of n members with
/// [`insert`](Self::insert), finds each of them, reads one from JSON or compares two takes
/// a time that grows as n log n. Moving or taking out a member, which moves those after
/// it, takes a time that grows as n.
#[derive(Clone, Default)]
pub struct Map {
    members: Vec<(String, Value)>,
    /// For an object of more than [`INDEXED`] members, the position of each name. In a
    /// box, it leaves a value of a tree the size that it has without an index.
    #[allow(clippy::box_collection, reason = "a value of a tree keeps its size")]
    index: Option<Box<BTreeMap<String, usize>>>,
}

/// The number of members above which an object keeps an index of its names: a value of a
/// protocol has fewer, and a search of so few names takes less than the index.
const INDEXED: usize = 32;

impl Map {
    /// An object without members.
    pub fn new() -> Self {
        Self::default()
    }

    /// An object without members, with room for `capacity` of them.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            members: Vec::with_capacity(capacity),
            index: None,
        }
    }

    /// How many members it has.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether it has none.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Take every member out.
    pub fn clear(&mut self) {
        self.members.clear();
        self.index = None;
    }

    /// The position of the member of this name.
    fn position(&self, name: &str) -> Option<usize> {
        match &self.index {
            Some(index) => index.get(name).copied(),
            None => self.members.iter().position(|(known, _)| known == name),
        }
    }

    /// Add a member of a name that the object does not have, at the end.
    fn push(&mut self, name: String, value: Value) {
        if let Some(index) = &mut self.index {
            index.insert(name.clone(), self.members.len());
        }
        self.members.push((name, value));
        if self.index.is_none() && self.members.len() > INDEXED {
            self.reindex(0);
        }
    }

    /// Give the index the positions of the members from `from`, which have moved, or make
    /// it, or leave it out for an object of [`INDEXED`] members or fewer.
    fn reindex(&mut self, from: usize) {
        if self.members.len() <= INDEXED {
            self.index = None;
            return;
        }
        let index = self.index.get_or_insert_with(Default::default);
        let from = if index.is_empty() { 0 } else { from };
        for (position, (name, _)) in self.members.iter().enumerate().skip(from) {
            match index.get_mut(name.as_str()) {
                Some(known) => *known = position,
                None => {
                    index.insert(name.clone(), position);
                }
            }
        }
    }

    /// The member of this name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.get_key_value(name).map(|(_, value)| value)
    }

    /// The member of this name, to change.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        let member = match &self.index {
            Some(index) => self.members.get_mut(*index.get(name)?),
            None => self.members.iter_mut().find(|(known, _)| known == name),
        };
        member.map(|(_, value)| value)
    }

    /// The member of this name, with its name as the object has it.
    pub fn get_key_value(&self, name: &str) -> Option<(&String, &Value)> {
        let member = match &self.index {
            Some(index) => self.members.get(*index.get(name)?),
            None => self.members.iter().find(|(known, _)| known == name),
        };
        member.map(|(name, value)| (name, value))
    }

    /// Whether it has a member of this name.
    pub fn contains_key(&self, name: &str) -> bool {
        self.position(name).is_some()
    }

    /// Give the member `name` the value `value`: at its place when the object has it,
    /// and at the end otherwise. The value that it had, if any.
    pub fn insert(&mut self, name: String, value: Value) -> Option<Value> {
        match self.get_mut(&name) {
            Some(was) => Some(core::mem::replace(was, value)),
            None => {
                self.push(name, value);
                None
            }
        }
    }

    /// Give the member `name` the value `value` at `position`, or at the end for a position
    /// past it: the member moves there when the object has it. The value that it had, if
    /// any.
    pub fn shift_insert(&mut self, position: usize, name: String, value: Value) -> Option<Value> {
        let was = self.shift_remove(&name);
        let position = position.min(self.members.len());
        self.members.insert(position, (name, value));
        self.reindex(position);
        was
    }

    /// Take the member `name` out, and leave the others in their order.
    pub fn shift_remove(&mut self, name: &str) -> Option<Value> {
        let position = self.position(name)?;
        let (name, value) = self.members.remove(position);
        if let Some(index) = &mut self.index {
            index.remove(&name);
        }
        self.reindex(position);
        Some(value)
    }

    /// Take the member `name` out, and leave the others in their order: as
    /// [`shift_remove`](Self::shift_remove), which keeps the order of the members, and
    /// not as the `swap_remove` of an `IndexMap`.
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        self.shift_remove(name)
    }

    /// The member `name`, to give it a value when it has none.
    pub fn entry(&mut self, name: impl Into<String>) -> Entry<'_> {
        let name = name.into();
        match self.position(&name) {
            Some(position) => Entry::Occupied(&mut self.members[position].1),
            None => Entry::Vacant(self, name),
        }
    }

    /// Keep the members for which `keep` says so.
    pub fn retain(&mut self, mut keep: impl FnMut(&String, &mut Value) -> bool) {
        let before = self.members.len();
        self.members.retain_mut(|(name, value)| keep(name, value));
        if self.members.len() != before {
            self.index = None;
            self.reindex(0);
        }
    }

    /// The members, in their order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&String, &Value)> + ExactSizeIterator {
        self.members.iter().map(|(name, value)| (name, value))
    }

    /// The members, in their order, to change their values.
    pub fn iter_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (&String, &mut Value)> + ExactSizeIterator {
        self.members.iter_mut().map(|(name, value)| (&*name, value))
    }

    /// The names of the members, in their order.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &String> + ExactSizeIterator {
        self.members.iter().map(|(name, _)| name)
    }

    /// The values of the members, in their order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Value> + ExactSizeIterator {
        self.members.iter().map(|(_, value)| value)
    }

    /// The values of the members, in their order, to change them.
    pub fn values_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = &mut Value> + ExactSizeIterator {
        self.members.iter_mut().map(|(_, value)| value)
    }

    /// The members, from the first, which have each their own name.
    pub(crate) fn from_members(members: Vec<(String, Value)>) -> Self {
        let mut map = Self {
            members,
            index: None,
        };
        map.reindex(0);
        map
    }

    /// The positions of the members, in the order of their names, and in that of the
    /// object for one name.
    fn by_name(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.members.len()).collect();
        order.sort_by(|a, b| self.members[*a].0.cmp(&self.members[*b].0));
        order
    }
}

/// A member of an object, which has a value or none yet: [`Map::entry`].
pub enum Entry<'m> {
    /// The member has this value.
    Occupied(&'m mut Value),
    /// The object has no member of this name.
    Vacant(&'m mut Map, String),
}

impl<'m> Entry<'m> {
    /// The value of the member, which is `value` when the object had none.
    pub fn or_insert(self, value: Value) -> &'m mut Value {
        self.or_insert_with(|| value)
    }

    /// The value of the member, which `value` gives when the object had none.
    pub fn or_insert_with(self, value: impl FnOnce() -> Value) -> &'m mut Value {
        match self {
            Entry::Occupied(value) => value,
            Entry::Vacant(map, name) => {
                map.push(name, value());
                &mut map
                    .members
                    .last_mut()
                    .expect("the member that was pushed")
                    .1
            }
        }
    }

    /// The value of the member, which is `null` when the object had none.
    pub fn or_default(self) -> &'m mut Value {
        self.or_insert(Value::Null)
    }
}

impl PartialEq for Map {
    /// Two objects are equal when they have the same members, whatever their order.
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        // In the same order, as two trees of one type are: a member after the other.
        let mut pairs = self.members.iter().zip(&other.members);
        if pairs.all(|((a, _), (b, _))| a == b) {
            return self
                .members
                .iter()
                .zip(&other.members)
                .all(|((_, a), (_, b))| a == b);
        }
        // Otherwise by the order of the names (each name is there once).
        let (mine, theirs) = (self.by_name(), other.by_name());
        mine.iter().zip(&theirs).all(|(a, b)| {
            let ((a_name, a), (b_name, b)) = (&self.members[*a], &other.members[*b]);
            a_name == b_name && a == b
        })
    }
}

impl Eq for Map {}

impl fmt::Debug for Map {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl<'m> IntoIterator for &'m Map {
    type Item = (&'m String, &'m Value);
    type IntoIter = core::iter::Map<
        core::slice::Iter<'m, (String, Value)>,
        fn(&'m (String, Value)) -> (&'m String, &'m Value),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.members.iter().map(|(name, value)| (name, value))
    }
}

impl<'m> IntoIterator for &'m mut Map {
    type Item = (&'m String, &'m mut Value);
    type IntoIter = core::iter::Map<
        core::slice::IterMut<'m, (String, Value)>,
        fn(&'m mut (String, Value)) -> (&'m String, &'m mut Value),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.members.iter_mut().map(|(name, value)| (&*name, value))
    }
}

impl IntoIterator for Map {
    type Item = (String, Value);
    type IntoIter = alloc::vec::IntoIter<(String, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.into_iter()
    }
}

impl FromIterator<(String, Value)> for Map {
    /// The members in the order of the iterator; a name that comes again gives its member
    /// the last value, at the place of the first.
    fn from_iter<I: IntoIterator<Item = (String, Value)>>(members: I) -> Self {
        let mut map = Map::new();
        map.extend(members);
        map
    }
}

impl Extend<(String, Value)> for Map {
    /// The members after those of the object, as [`insert`](Map::insert) adds them one
    /// after the other.
    fn extend<I: IntoIterator<Item = (String, Value)>>(&mut self, members: I) {
        for (name, value) in members {
            self.insert(name, value);
        }
    }
}

impl ops::Index<&str> for Map {
    type Output = Value;

    /// # Panics
    ///
    /// When the object has no member of this name.
    fn index(&self, name: &str) -> &Value {
        self.get(name).expect("no member of this name")
    }
}

impl ops::IndexMut<&str> for Map {
    /// # Panics
    ///
    /// When the object has no member of this name.
    fn index_mut(&mut self, name: &str) -> &mut Value {
        self.get_mut(name).expect("no member of this name")
    }
}

// ── The value ───────────────────────────────────────────────────────────────

static NULL: Value = Value::Null;

impl Value {
    /// Whether it is `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Whether it is `true` or `false`.
    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Whether it is a number.
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Whether it is a whole number that a `u64` holds.
    pub fn is_u64(&self) -> bool {
        matches!(self, Value::Number(number) if number.is_u64())
    }

    /// Whether it is a whole number that an `i64` holds.
    pub fn is_i64(&self) -> bool {
        matches!(self, Value::Number(number) if number.is_i64())
    }

    /// Whether it is a number with a fraction or an exponent.
    pub fn is_f64(&self) -> bool {
        matches!(self, Value::Number(number) if number.is_f64())
    }

    /// Whether it is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Whether it is a list.
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Whether it is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    /// `()` for `null`.
    pub fn as_null(&self) -> Option<()> {
        self.is_null().then_some(())
    }

    /// The boolean that it is.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(boolean) => Some(*boolean),
            _ => None,
        }
    }

    /// The number that it is.
    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Value::Number(number) => Some(number),
            _ => None,
        }
    }

    /// The whole number that it is, when a `u64` holds it.
    pub fn as_u64(&self) -> Option<u64> {
        self.as_number().and_then(Number::as_u64)
    }

    /// The whole number that it is, when an `i64` holds it.
    pub fn as_i64(&self) -> Option<i64> {
        self.as_number().and_then(Number::as_i64)
    }

    /// The number that it is, as an `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        self.as_number().and_then(Number::as_f64)
    }

    /// The string that it is.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }

    /// The list that it is.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(entries) => Some(entries),
            _ => None,
        }
    }

    /// The list that it is, to change.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(entries) => Some(entries),
            _ => None,
        }
    }

    /// The object that it is.
    pub fn as_object(&self) -> Option<&Map> {
        match self {
            Value::Object(members) => Some(members),
            _ => None,
        }
    }

    /// The object that it is, to change.
    pub fn as_object_mut(&mut self) -> Option<&mut Map> {
        match self {
            Value::Object(members) => Some(members),
            _ => None,
        }
    }

    /// The member of an object by its name, or the entry of a list by its position.
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        index.index_into(self)
    }

    /// The member of an object by its name, or the entry of a list by its position, to
    /// change.
    pub fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut Value> {
        index.index_into_mut(self)
    }

    /// The value at a JSON pointer (RFC 6901), as `"/message/protocolIEs/0"`: the empty
    /// pointer is the value itself.
    pub fn pointer(&self, pointer: &str) -> Option<&Value> {
        if pointer.is_empty() {
            return Some(self);
        }
        let rest = pointer.strip_prefix('/')?;
        rest.split('/').try_fold(self, |value, token| {
            let token = token.replace("~1", "/").replace("~0", "~");
            match value {
                Value::Object(members) => members.get(&token),
                Value::Array(entries) => entries.get(super::position(&token)?),
                _ => None,
            }
        })
    }

    /// The value at a JSON pointer, to change.
    pub fn pointer_mut(&mut self, pointer: &str) -> Option<&mut Value> {
        if pointer.is_empty() {
            return Some(self);
        }
        let rest = pointer.strip_prefix('/')?;
        rest.split('/').try_fold(self, |value, token| {
            let token = token.replace("~1", "/").replace("~0", "~");
            match value {
                Value::Object(members) => members.get_mut(&token),
                Value::Array(entries) => entries.get_mut(super::position(&token)?),
                _ => None,
            }
        })
    }

    /// The value, which `null` takes the place of.
    pub fn take(&mut self) -> Value {
        core::mem::take(self)
    }
}

/// What indexes a value: the name of a member, or the position of an entry.
pub trait Index: private::Sealed {
    #[doc(hidden)]
    fn index_into<'v>(&self, value: &'v Value) -> Option<&'v Value>;
    #[doc(hidden)]
    fn index_into_mut<'v>(&self, value: &'v mut Value) -> Option<&'v mut Value>;
    #[doc(hidden)]
    fn index_or_insert<'v>(&self, value: &'v mut Value) -> &'v mut Value;
}

mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for alloc::string::String {}
    impl<T: Sealed + ?Sized> Sealed for &T {}
}

impl Index for usize {
    fn index_into<'v>(&self, value: &'v Value) -> Option<&'v Value> {
        value.as_array()?.get(*self)
    }

    fn index_into_mut<'v>(&self, value: &'v mut Value) -> Option<&'v mut Value> {
        value.as_array_mut()?.get_mut(*self)
    }

    fn index_or_insert<'v>(&self, value: &'v mut Value) -> &'v mut Value {
        match value {
            Value::Array(entries) => {
                let length = entries.len();
                entries.get_mut(*self).unwrap_or_else(|| {
                    panic!("cannot access index {self} of a list of length {length}")
                })
            }
            _ => panic!("cannot access index {self} of a value that is no list"),
        }
    }
}

impl Index for str {
    fn index_into<'v>(&self, value: &'v Value) -> Option<&'v Value> {
        value.as_object()?.get(self)
    }

    fn index_into_mut<'v>(&self, value: &'v mut Value) -> Option<&'v mut Value> {
        value.as_object_mut()?.get_mut(self)
    }

    fn index_or_insert<'v>(&self, value: &'v mut Value) -> &'v mut Value {
        if value.is_null() {
            *value = Value::Object(Map::new());
        }
        match value {
            Value::Object(members) => members.entry(self.to_owned()).or_default(),
            _ => panic!("cannot access member {self:?} of a value that is no object"),
        }
    }
}

impl Index for String {
    fn index_into<'v>(&self, value: &'v Value) -> Option<&'v Value> {
        self.as_str().index_into(value)
    }

    fn index_into_mut<'v>(&self, value: &'v mut Value) -> Option<&'v mut Value> {
        self.as_str().index_into_mut(value)
    }

    fn index_or_insert<'v>(&self, value: &'v mut Value) -> &'v mut Value {
        self.as_str().index_or_insert(value)
    }
}

impl<T: Index + ?Sized> Index for &T {
    fn index_into<'v>(&self, value: &'v Value) -> Option<&'v Value> {
        (**self).index_into(value)
    }

    fn index_into_mut<'v>(&self, value: &'v mut Value) -> Option<&'v mut Value> {
        (**self).index_into_mut(value)
    }

    fn index_or_insert<'v>(&self, value: &'v mut Value) -> &'v mut Value {
        (**self).index_or_insert(value)
    }
}

impl<I: Index> ops::Index<I> for Value {
    type Output = Value;

    /// The member or the entry, and `null` where there is none.
    fn index(&self, index: I) -> &Value {
        index.index_into(self).unwrap_or(&NULL)
    }
}

impl<I: Index> ops::IndexMut<I> for Value {
    /// The member, which an object takes as `null` when it has none, and which `null`
    /// takes as an object with that member; or the entry.
    ///
    /// # Panics
    ///
    /// For a position past the end of a list, and for a value of another kind.
    fn index_mut(&mut self, index: I) -> &mut Value {
        index.index_or_insert(self)
    }
}

// ── Equality ────────────────────────────────────────────────────────────────

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialEq<serde_json::Value> for Value {
    /// Whether the two are the same JSON: objects whatever the order of their members.
    fn eq(&self, other: &serde_json::Value) -> bool {
        match (self, other) {
            (Value::Null, serde_json::Value::Null) => true,
            (Value::Bool(a), serde_json::Value::Bool(b)) => a == b,
            (Value::Number(a), serde_json::Value::Number(b)) => a == b,
            (Value::String(a), serde_json::Value::String(b)) => a == b,
            (Value::Array(a), serde_json::Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a == b)
            }
            (Value::Object(a), serde_json::Value::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(name, value)| b.get(name).is_some_and(|b| value == b))
            }
            _ => false,
        }
    }
}

impl PartialEq<Value> for serde_json::Value {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

macro_rules! equal_to {
    ($($ty:ty => $as:ident as $wide:ty),* $(,)?) => {$(
        impl PartialEq<$ty> for Value {
            fn eq(&self, other: &$ty) -> bool {
                self.$as().is_some_and(|value| value == *other as $wide)
            }
        }
        impl PartialEq<$ty> for &Value {
            fn eq(&self, other: &$ty) -> bool {
                (**self) == *other
            }
        }
        impl PartialEq<Value> for $ty {
            fn eq(&self, other: &Value) -> bool {
                other == self
            }
        }
    )*};
}

equal_to! {
    u8 => as_u64 as u64, u16 => as_u64 as u64, u32 => as_u64 as u64, u64 => as_u64 as u64,
    usize => as_u64 as u64, i8 => as_i64 as i64, i16 => as_i64 as i64, i32 => as_i64 as i64,
    i64 => as_i64 as i64, isize => as_i64 as i64, f64 => as_f64 as f64, bool => as_bool as bool,
}

impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<String> for Value {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}

impl PartialEq<Value> for str {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

impl PartialEq<Value> for &str {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

impl PartialEq<Value> for String {
    fn eq(&self, other: &Value) -> bool {
        other == self
    }
}

// ── Conversions ─────────────────────────────────────────────────────────────

macro_rules! from_number {
    ($($ty:ty),* $(,)?) => {$(
        impl From<$ty> for Value {
            fn from(number: $ty) -> Self {
                Value::Number(number.into())
            }
        }
        impl From<&$ty> for Value {
            fn from(number: &$ty) -> Self {
                Value::Number((*number).into())
            }
        }
    )*};
}

from_number!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

impl From<&f64> for Value {
    fn from(number: &f64) -> Self {
        Value::from(*number)
    }
}

impl From<&bool> for Value {
    fn from(boolean: &bool) -> Self {
        Value::Bool(*boolean)
    }
}

impl From<&&str> for Value {
    fn from(text: &&str) -> Self {
        Value::String((*text).into())
    }
}

impl From<f64> for Value {
    /// The number, and `null` for one that JSON has not: an infinity or no number.
    fn from(number: f64) -> Self {
        Number::from_f64(number).map_or(Value::Null, Value::Number)
    }
}

impl From<bool> for Value {
    fn from(boolean: bool) -> Self {
        Value::Bool(boolean)
    }
}

impl From<Number> for Value {
    fn from(number: Number) -> Self {
        Value::Number(number)
    }
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Value::String(text)
    }
}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Value::String(text.to_string())
    }
}

impl From<&String> for Value {
    fn from(text: &String) -> Self {
        Value::String(text.clone())
    }
}

impl From<&Value> for Value {
    fn from(value: &Value) -> Self {
        value.clone()
    }
}

impl<T: Clone + Into<Value>> From<&[T]> for Value {
    fn from(entries: &[T]) -> Self {
        Value::Array(entries.iter().cloned().map(Into::into).collect())
    }
}

impl<T: Into<Value>, const N: usize> From<[T; N]> for Value {
    fn from(entries: [T; N]) -> Self {
        Value::Array(entries.into_iter().map(Into::into).collect())
    }
}

impl From<Map> for Value {
    fn from(members: Map) -> Self {
        Value::Object(members)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(entries: Vec<T>) -> Self {
        Value::Array(entries.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    /// The value, and `null` for none.
    fn from(value: Option<T>) -> Self {
        value.map_or(Value::Null, Into::into)
    }
}

impl From<()> for Value {
    fn from((): ()) -> Self {
        Value::Null
    }
}

impl<T: Into<Value>> FromIterator<T> for Value {
    /// A list of the values.
    fn from_iter<I: IntoIterator<Item = T>>(entries: I) -> Self {
        Value::Array(entries.into_iter().map(Into::into).collect())
    }
}

impl From<serde_json::Value> for Value {
    /// The same JSON. The members of an object come in the order in which
    /// `serde_json::Map` iterates them: by their names, unless a crate of the program
    /// turns on `serde_json/preserve_order`.
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(boolean) => Value::Bool(boolean),
            serde_json::Value::Number(number) => Value::Number(number),
            serde_json::Value::String(text) => Value::String(text),
            serde_json::Value::Array(entries) => {
                Value::Array(entries.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(members) => Value::Object(Map::from_members(
                members
                    .into_iter()
                    .map(|(name, value)| (name, Value::from(value)))
                    .collect(),
            )),
        }
    }
}

impl From<&serde_json::Value> for Value {
    fn from(value: &serde_json::Value) -> Self {
        Value::from(value.clone())
    }
}

impl From<Value> for serde_json::Value {
    /// The same JSON. A `serde_json::Map` keeps the order of the members only where a
    /// crate of the program turns on `serde_json/preserve_order`, and sorts them by their
    /// names otherwise.
    fn from(value: Value) -> Self {
        match value {
            Value::Null => serde_json::Value::Null,
            Value::Bool(boolean) => serde_json::Value::Bool(boolean),
            Value::Number(number) => serde_json::Value::Number(number),
            Value::String(text) => serde_json::Value::String(text),
            Value::Array(entries) => {
                serde_json::Value::Array(entries.into_iter().map(Into::into).collect())
            }
            Value::Object(members) => serde_json::Value::Object(
                members
                    .into_iter()
                    .map(|(name, value)| (name, value.into()))
                    .collect(),
            ),
        }
    }
}

impl From<&Value> for serde_json::Value {
    fn from(value: &Value) -> Self {
        value.clone().into()
    }
}

// ── Text ────────────────────────────────────────────────────────────────────

impl core::str::FromStr for Value {
    type Err = serde_json::Error;

    /// The tree that a JSON text writes, its members in the order of the text. A name
    /// that an object has twice is refused.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(text)
    }
}

impl fmt::Display for Value {
    /// The JSON text of the tree, its members in their order: compact, or indented with
    /// `{:#}`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match formatter.alternate() {
            true => serde_json::to_string_pretty(self),
            false => serde_json::to_string(self),
        };
        formatter.write_str(&text.map_err(|_| fmt::Error)?)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => formatter.write_str("Null"),
            Value::Bool(boolean) => write!(formatter, "Bool({boolean})"),
            Value::Number(number) => write!(formatter, "Number({number})"),
            Value::String(text) => write!(formatter, "String({text:?})"),
            Value::Array(entries) => {
                formatter.write_str("Array ")?;
                formatter.debug_list().entries(entries).finish()
            }
            Value::Object(members) => {
                formatter.write_str("Object ")?;
                fmt::Debug::fmt(members, formatter)
            }
        }
    }
}

// ── serde ───────────────────────────────────────────────────────────────────

impl serde_core::Serialize for Value {
    /// The value in any format of serde, the members of an object in their order.
    fn serialize<S: serde_core::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde_core::ser::{SerializeMap, SerializeSeq};
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(boolean) => serializer.serialize_bool(*boolean),
            Value::Number(number) => number.serialize(serializer),
            Value::String(text) => serializer.serialize_str(text),
            Value::Array(entries) => {
                let mut list = serializer.serialize_seq(Some(entries.len()))?;
                for entry in entries {
                    list.serialize_element(entry)?;
                }
                list.end()
            }
            Value::Object(members) => {
                let mut object = serializer.serialize_map(Some(members.len()))?;
                for (name, value) in members {
                    object.serialize_entry(name, value)?;
                }
                object.end()
            }
        }
    }
}

impl<'de> serde_core::Deserialize<'de> for Value {
    /// The value that a self-describing format writes, the members of an object in the
    /// order in which it writes them. A name that an object has twice is refused: JSON
    /// leaves what it means open, and a reader of a tree takes no guess.
    fn deserialize<D: serde_core::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Visitor)
    }
}

struct Visitor;

impl<'de> serde_core::de::Visitor<'de> for Visitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, boolean: bool) -> Result<Value, E> {
        Ok(Value::Bool(boolean))
    }

    fn visit_i64<E>(self, number: i64) -> Result<Value, E> {
        Ok(number.into())
    }

    fn visit_u64<E>(self, number: u64) -> Result<Value, E> {
        Ok(number.into())
    }

    fn visit_i128<E: serde_core::de::Error>(self, number: i128) -> Result<Value, E> {
        match (i64::try_from(number), u64::try_from(number)) {
            (Ok(number), _) => Ok(number.into()),
            (_, Ok(number)) => Ok(number.into()),
            _ => Err(E::custom("a number that 64 bits do not hold")),
        }
    }

    fn visit_u128<E: serde_core::de::Error>(self, number: u128) -> Result<Value, E> {
        u64::try_from(number)
            .map(Value::from)
            .map_err(|_| E::custom("a number that 64 bits do not hold"))
    }

    fn visit_f64<E>(self, number: f64) -> Result<Value, E> {
        Ok(number.into())
    }

    fn visit_str<E>(self, text: &str) -> Result<Value, E> {
        Ok(Value::String(text.to_string()))
    }

    fn visit_string<E>(self, text: String) -> Result<Value, E> {
        Ok(Value::String(text))
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D: serde_core::Deserializer<'de>>(self, value: D) -> Result<Value, D::Error> {
        serde_core::Deserialize::deserialize(value)
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: serde_core::de::SeqAccess<'de>>(self, mut list: A) -> Result<Value, A::Error> {
        let mut entries = Vec::with_capacity(list.size_hint().unwrap_or(0).min(4096));
        while let Some(entry) = list.next_element()? {
            entries.push(entry);
        }
        Ok(Value::Array(entries))
    }

    fn visit_map<A: serde_core::de::MapAccess<'de>>(
        self,
        mut object: A,
    ) -> Result<Value, A::Error> {
        use serde_core::de::Error as _;
        let mut members: Vec<(String, Value)> =
            Vec::with_capacity(object.size_hint().unwrap_or(0).min(4096));
        while let Some((name, value)) = object.next_entry::<String, Value>()? {
            members.push((name, value));
        }
        // A name that comes twice, found in the order of the names: a time that grows
        // as n log n, for an object of any size.
        let mut names: Vec<&str> = members.iter().map(|(name, _)| name.as_str()).collect();
        names.sort_unstable();
        if let Some(twice) = names.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(A::Error::custom(alloc::format!(
                "the member {:?} is written twice in an object",
                twice[0]
            )));
        }
        Ok(Value::Object(Map::from_members(members)))
    }
}
