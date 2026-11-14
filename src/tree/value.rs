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

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops;

pub use serde_json::Number;

/// A tree: a JSON value whose objects keep their members in the order in which they were
/// written, which is that of the specification for the tree of a message.
///
/// It has the variants of `serde_json::Value` and its usual calls, and converts to and
/// from one. Two objects are equal when they have the same members, whatever their
/// order, as two JSON objects are. It prints as JSON, its members in their order, and
/// `{:#}` prints it indented. A member is `tree["name"]` and an entry `tree[0]`, which
/// are `null` where the tree has none.
///
/// ```
/// use oxirush_utils::tree;
/// use oxirush_utils::tree::{Map, Value};
///
/// let mut slice = Map::new();
/// slice.insert("sst".into(), 1.into());
/// slice.insert("sd".into(), "0000ff".into());
/// let tree = Value::Object(slice);
/// assert_eq!(tree.to_string(), r#"{"sst":1,"sd":"0000ff"}"#);
/// assert_eq!(tree, tree!({"sd": "0000ff", "sst": 1}));
/// assert_eq!(tree["sst"].as_u64(), Some(1));
/// assert!(tree["sd"]["what"].is_null());
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
    /// A string: text of its own, or a name that the program holds, as the name of a
    /// value of an enumeration, which is not copied.
    String(Cow<'static, str>),
    /// A list.
    Array(Vec<Value>),
    /// An object, its members in the order in which they were written.
    Object(Map),
}

/// The members of an object of a tree, in the order in which they were written: a member
/// that is added goes at the end, one that is given another value keeps its place, and
/// one that is taken out leaves the others in theirs.
///
/// It is a list, which is searched for a name: a value of a protocol has a few dozen
/// members at most, as the 84 bits of a 5GMM capability, and a search of so few names
/// takes less than a map. An object that is read from a format has 4096 members at most.
/// A name is text of its own or one that the program holds, as the name of a member of
/// a type, which is not copied.
#[derive(Clone, Default)]
pub struct Map {
    members: Vec<(Cow<'static, str>, Value)>,
}

/// The most members that an object that is read has: far more than a value of a protocol
/// has, and few enough that a search of each name stays short.
const MOST_MEMBERS: usize = 4096;

impl Map {
    /// An object without members.
    pub fn new() -> Self {
        Self::default()
    }

    /// An object without members, with room for `capacity` of them.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            members: Vec::with_capacity(capacity),
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
    }

    /// The position of the member of this name.
    fn position(&self, name: &str) -> Option<usize> {
        self.members.iter().position(|(known, _)| known == name)
    }

    /// The member of this name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.get_key_value(name).map(|(_, value)| value)
    }

    /// The member of this name, to change.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        let member = self.members.iter_mut().find(|(known, _)| known == name);
        member.map(|(_, value)| value)
    }

    /// The member of this name, with its name as the object has it.
    pub fn get_key_value(&self, name: &str) -> Option<(&str, &Value)> {
        let member = self.members.iter().find(|(known, _)| known == name);
        member.map(|(name, value)| (&**name, value))
    }

    /// Whether it has a member of this name.
    pub fn contains_key(&self, name: &str) -> bool {
        self.position(name).is_some()
    }

    /// Give the member `name` the value `value`: at its place when the object has it,
    /// and at the end otherwise. The value that it had, if any.
    pub fn insert(&mut self, name: Cow<'static, str>, value: Value) -> Option<Value> {
        match self.get_mut(&name) {
            Some(was) => Some(core::mem::replace(was, value)),
            None => {
                self.members.push((name, value));
                None
            }
        }
    }

    /// Add the member `name` at the end without searching for it, for a name that the
    /// object does not have: the members of a value that a type writes one after the
    /// other. A name that the object has is a logic error: a debug build panics, and
    /// otherwise the object has the name twice, of which `get` and `insert` see the
    /// first.
    #[inline]
    pub fn push(&mut self, name: Cow<'static, str>, value: Value) {
        debug_assert!(
            self.position(&name).is_none(),
            "the object has a member {name:?} already"
        );
        self.members.push((name, value));
    }

    /// Put back at `position` a member that was taken out of it, for the undo of an edit.
    pub(crate) fn insert_at(&mut self, position: usize, name: Cow<'static, str>, value: Value) {
        let position = position.min(self.members.len());
        self.members.insert(position, (name, value));
    }

    /// Take the member `name` out, and leave the others in their order.
    pub fn shift_remove(&mut self, name: &str) -> Option<Value> {
        let position = self.position(name)?;
        Some(self.members.remove(position).1)
    }

    /// The member `name`, to give it a value when it has none.
    pub fn entry(&mut self, name: impl Into<Cow<'static, str>>) -> Entry<'_> {
        let name = name.into();
        match self.position(&name) {
            Some(position) => Entry::Occupied(&mut self.members[position].1),
            None => Entry::Vacant(self, name),
        }
    }

    /// Keep the members for which `keep` says so.
    pub fn retain(&mut self, mut keep: impl FnMut(&str, &mut Value) -> bool) {
        self.members.retain_mut(|(name, value)| keep(name, value));
    }

    /// The members, in their order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&str, &Value)> + ExactSizeIterator {
        self.members.iter().map(|(name, value)| (&**name, value))
    }

    /// The members, in their order, to change their values.
    pub fn iter_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (&str, &mut Value)> + ExactSizeIterator {
        self.members
            .iter_mut()
            .map(|(name, value)| (&**name, value))
    }

    /// The names of the members, in their order.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &str> + ExactSizeIterator {
        self.members.iter().map(|(name, _)| &**name)
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
    pub(crate) fn from_members(members: Vec<(Cow<'static, str>, Value)>) -> Self {
        Self { members }
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
    Vacant(&'m mut Map, Cow<'static, str>),
}

impl<'m> Entry<'m> {
    /// The value of the member, which is `value` when the object had none.
    pub fn or_insert(self, value: Value) -> &'m mut Value {
        match self {
            Entry::Occupied(value) => value,
            Entry::Vacant(map, name) => {
                map.members.push((name, value));
                &mut map
                    .members
                    .last_mut()
                    .expect("the member that was pushed")
                    .1
            }
        }
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

type Member = (Cow<'static, str>, Value);

impl<'m> IntoIterator for &'m Map {
    type Item = (&'m str, &'m Value);
    type IntoIter =
        core::iter::Map<core::slice::Iter<'m, Member>, fn(&'m Member) -> (&'m str, &'m Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.iter().map(|(name, value)| (&**name, value))
    }
}

impl<'m> IntoIterator for &'m mut Map {
    type Item = (&'m str, &'m mut Value);
    type IntoIter = core::iter::Map<
        core::slice::IterMut<'m, Member>,
        fn(&'m mut Member) -> (&'m str, &'m mut Value),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.members
            .iter_mut()
            .map(|(name, value)| (&**name, value))
    }
}

impl IntoIterator for Map {
    type Item = Member;
    type IntoIter = alloc::vec::IntoIter<Member>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.into_iter()
    }
}

impl FromIterator<Member> for Map {
    /// The members in the order of the iterator; a name that comes again gives its member
    /// the last value, at the place of the first.
    fn from_iter<I: IntoIterator<Item = Member>>(members: I) -> Self {
        let members = members.into_iter();
        let mut map = Map::with_capacity(members.size_hint().0);
        map.extend(members);
        map
    }
}

impl Extend<Member> for Map {
    /// The members after those of the object, as [`insert`](Map::insert) adds them one
    /// after the other.
    fn extend<I: IntoIterator<Item = Member>>(&mut self, members: I) {
        let members = members.into_iter();
        self.members.reserve(members.size_hint().0);
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

    /// The member of an object by its name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.as_object()?.get(name)
    }

    /// The member of an object by its name, to change.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.as_object_mut()?.get_mut(name)
    }
}

impl ops::Index<&str> for Value {
    type Output = Value;

    /// The member of an object, and `null` where there is none.
    fn index(&self, name: &str) -> &Value {
        self.get(name).unwrap_or(&NULL)
    }
}

impl ops::IndexMut<&str> for Value {
    /// The member of an object, which the object takes as `null` when it has none, and
    /// which `null` takes as an object with that member.
    ///
    /// # Panics
    ///
    /// For a value that is neither an object nor `null`.
    fn index_mut(&mut self, name: &str) -> &mut Value {
        if self.is_null() {
            *self = Value::Object(Map::new());
        }
        match self {
            Value::Object(members) => match members.position(name) {
                Some(position) => &mut members.members[position].1,
                None => members.entry(String::from(name)).or_insert(Value::Null),
            },
            _ => panic!("cannot access member {name:?} of a value that is no object"),
        }
    }
}

impl ops::Index<usize> for Value {
    type Output = Value;

    /// The entry of a list, and `null` where there is none.
    fn index(&self, position: usize) -> &Value {
        self.as_array()
            .and_then(|entries| entries.get(position))
            .unwrap_or(&NULL)
    }
}

impl ops::IndexMut<usize> for Value {
    /// The entry of a list.
    ///
    /// # Panics
    ///
    /// For a position past the end of the list, and for a value that is no list.
    fn index_mut(&mut self, position: usize) -> &mut Value {
        match self {
            Value::Array(entries) => {
                let length = entries.len();
                entries.get_mut(position).unwrap_or_else(|| {
                    panic!("cannot access index {position} of a list of length {length}")
                })
            }
            _ => panic!("cannot access index {position} of a value that is no list"),
        }
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

impl From<&&'static str> for Value {
    /// The text, which is not copied.
    fn from(text: &&'static str) -> Self {
        Value::String(Cow::Borrowed(text))
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
        Value::String(Cow::Owned(text))
    }
}

impl From<&'static str> for Value {
    /// The text, which is not copied: a literal, or a name that the program holds. Text
    /// that lives less long is a `String` first, or goes through [`text`](super::text).
    fn from(text: &'static str) -> Self {
        Value::String(Cow::Borrowed(text))
    }
}

impl From<Cow<'static, str>> for Value {
    fn from(text: Cow<'static, str>) -> Self {
        Value::String(text)
    }
}

impl From<&String> for Value {
    fn from(text: &String) -> Self {
        Value::String(Cow::Owned(text.clone()))
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
            serde_json::Value::String(text) => Value::String(Cow::Owned(text)),
            serde_json::Value::Array(entries) => {
                Value::Array(entries.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(members) => Value::Object(Map::from_members(
                members
                    .into_iter()
                    .map(|(name, value)| (Cow::Owned(name), Value::from(value)))
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
            Value::String(text) => serde_json::Value::String(text.into_owned()),
            Value::Array(entries) => {
                serde_json::Value::Array(entries.into_iter().map(Into::into).collect())
            }
            Value::Object(members) => serde_json::Value::Object(
                members
                    .into_iter()
                    .map(|(name, value)| (name.into_owned(), value.into()))
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
        Ok(Value::String(Cow::Owned(text.into())))
    }

    fn visit_string<E>(self, text: String) -> Result<Value, E> {
        Ok(Value::String(Cow::Owned(text)))
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
        let mut members: Vec<Member> =
            Vec::with_capacity(object.size_hint().unwrap_or(0).min(4096));
        while let Some((name, value)) = object.next_entry::<String, Value>()? {
            if members.len() == MOST_MEMBERS {
                return Err(A::Error::custom(alloc::format!(
                    "an object has {MOST_MEMBERS} members at most"
                )));
            }
            members.push((Cow::Owned(name), value));
        }
        // A name that comes twice, found in the order of the names.
        let mut names: Vec<&str> = members.iter().map(|(name, _)| &**name).collect();
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
