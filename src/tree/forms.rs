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

//! The forms of the plain values of a tree: octets, numbers, names, bit strings.

use super::value::{Map, Number, Value};
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

// ── Octets ──────────────────────────────────────────────────────────────────

/// Octets in hexadecimal, in lower case: the form of octets in a tree.
///
/// ```
/// use oxirush_utils::tree;
///
/// assert_eq!(tree::hex(&[0x02, 0xf8, 0x39]), "02f839");
/// assert_eq!(tree::unhex("02F839"), Some(vec![0x02, 0xf8, 0x39]));
/// ```
pub fn hex(octets: &[u8]) -> String {
    // A digit is its half octet after `0`, and 39 further for one after `9`, which are
    // the characters between `9` and `a`. The loop has no table and no branch, so the
    // compiler writes it for several octets at once.
    let digit = |half: u8| half + b'0' + u8::from(half > 9) * (b'a' - b'9' - 1);
    let mut text = vec![0; octets.len() * 2];
    let (pairs, _) = text.as_chunks_mut::<2>();
    for (pair, octet) in pairs.iter_mut().zip(octets) {
        *pair = [digit(octet >> 4), digit(octet & 15)];
    }
    // The digits are ASCII.
    String::from_utf8(text).unwrap_or_default()
}

/// The octets that `text` writes in hexadecimal: two digits for each, in either case,
/// and nothing else. `None` for a prefix, a separator, or an odd number of digits.
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    /// What is no digit: a value that no half octet has.
    const NONE: u8 = 0x80;
    // The value of a digit, without a branch that leaves the loop: what is no digit is
    // told after it. A letter is taken in lower case, which differs from upper case by
    // one bit.
    let digit = |digit: u8| {
        let (number, letter) = (digit.wrapping_sub(b'0'), (digit | 0x20).wrapping_sub(b'a'));
        match (number < 10, letter < 6) {
            (true, _) => number,
            (false, true) => letter + 10,
            (false, false) => NONE,
        }
    };
    let (pairs, rest) = text.as_bytes().as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    let mut octets = vec![0; pairs.len()];
    let mut other = 0;
    for (octet, [high, low]) in octets.iter_mut().zip(pairs) {
        let (high, low) = (digit(*high), digit(*low));
        other |= high | low;
        *octet = high << 4 | low & 15;
    }
    (other & NONE == 0).then_some(octets)
}

// ── Names ───────────────────────────────────────────────────────────────────

/// The letters and the digits of a name, in lower case.
fn letters(name: &str) -> impl Iterator<Item = u8> + '_ {
    let letters = name.bytes().filter(u8::is_ascii_alphanumeric);
    letters.map(|letter| letter.to_ascii_lowercase())
}

/// Whether two names are the same: they have the same ASCII letters and digits, whatever
/// their case and whatever is between them. A name without a letter or a digit is no
/// name, and is the same as none.
///
/// ```
/// use oxirush_utils::tree::same_name;
///
/// assert!(same_name("5GMM cause", "5gmm_cause") && same_name("AMF-UE-NGAP-ID", "amfUeNgapId"));
/// assert!(!same_name("pLMNIdentity", "plmn") && !same_name("-", "_"));
/// ```
pub fn same_name(a: &str, b: &str) -> bool {
    letters(a).next().is_some() && letters(a).eq(letters(b))
}

/// Whether two names are written alike: the same characters, but for their case and for
/// which of `-`, `_` and space stands between their words.
fn alike(a: &str, b: &str) -> bool {
    let separator = |c: u8| matches!(c, b'-' | b'_' | b' ');
    a.len() == b.len()
        && (a.bytes().zip(b.bytes()))
            .all(|(a, b)| a.eq_ignore_ascii_case(&b) || (separator(a) && separator(b)))
}

/// The position among `names` of the one that `name` is. The name itself wins, then one
/// that is written alike, but for its case and its separators, then the first that is
/// the [same name](same_name).
///
/// ```
/// use oxirush_utils::tree::named;
///
/// let names = ["s-nssai", "snssai", "sNSSAI"];
/// assert_eq!(named(&names, "snssai"), Some(1));
/// assert_eq!(named(&names, "S_NSSAI"), Some(0));
/// assert_eq!(named(&names, "s.nssai"), Some(0));
/// assert_eq!(named(&names, "nssai"), None);
/// ```
pub fn named<T: AsRef<str>>(names: &[T], name: &str) -> Option<usize> {
    let position = |is: &dyn Fn(&str) -> bool| names.iter().position(|known| is(known.as_ref()));
    position(&|known| known == name)
        .or_else(|| position(&|known| alike(known, name)))
        .or_else(|| position(&|known| same_name(known, name)))
}

/// The name of `members` that `name` is, as [`named`] finds it.
pub(crate) fn key<'m>(members: &'m Map, name: &str) -> Option<&'m str> {
    // The name as it is written is the usual case, and is found without a search.
    if let Some((known, _)) = members.get_key_value(name) {
        return Some(known);
    }
    let alike = members.keys().find(|known| alike(known, name));
    let found = alike.or_else(|| members.keys().find(|known| same_name(known, name)));
    found.map(String::as_str)
}

// ── From a value to its form ────────────────────────────────────────────────

/// The form of a number without a sign: a JSON number, exact up to 2^64 - 1.
pub fn unsigned(value: u64) -> Value {
    Value::Number(Number::from(value))
}

/// The form of a number with a sign: a JSON number, exact from -2^63.
pub fn signed(value: i64) -> Value {
    Value::Number(Number::from(value))
}

/// The form of octets: [hexadecimal](hex) in lower case.
pub fn octets(octets: &[u8]) -> Value {
    Value::String(hex(octets))
}

/// The form of text.
pub fn text(text: &str) -> Value {
    Value::String(text.to_string())
}

/// The form of a string of bits whose length its type does not say: its octets, with
/// zero bits after the last, and how many bits it has.
///
/// A family writes a string of bits whose length is that of its type as its
/// [`octets`] alone.
///
/// ```
/// use oxirush_utils::tree;
///
/// assert_eq!(tree::bits(&[0xc0], 3), tree!({"value": "c0", "length": 3}));
/// ```
pub fn bits(octets: &[u8], length: usize) -> Value {
    let mut members = Map::new();
    members.insert("value".into(), Value::String(hex(octets)));
    members.insert("length".into(), Value::Number(Number::from(length)));
    Value::Object(members)
}

/// The form of a value that is one of several alternatives, with content: an object with
/// one member, the name of the alternative with its value. An alternative without
/// content is its name alone, as [`text`].
pub fn alternative(name: &str, value: Value) -> Value {
    let mut members = Map::new();
    members.insert(name.to_string(), value);
    Value::Object(members)
}

// ── How a written value compares ────────────────────────────────────────────

/// How the value at a place of a tree is written, for a tool that compares it with a
/// value that someone wrote: only the family of the tree knows that a string at a place
/// is octets, or one name of a set.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// A value that is compared as it is written: text, a number, a boolean.
    Plain,
    /// Octets in hexadecimal, which a tree shows in lower case and reads in either.
    Octets,
    /// One of these names, which a tree shows as it is here and reads [however it is
    /// written](same_name).
    Named(&'static [&'static str]),
}

/// A value that someone wrote, as a tree shows it at a place of the form `form`: what a
/// comparison with the value of the tree takes in its place, so that it does not depend
/// on how the writer spelled it.
///
/// Hexadecimal digits go to lower case at a place of octets, and a name becomes the one
/// of its set that it is. Anything else is as it was written: text that is not octets
/// stays in its case.
///
/// ```
/// use oxirush_utils::tree;
/// use oxirush_utils::tree::{Form, written_as};
///
/// // The tree shows octets in lower case, whatever the writer chose
/// assert_eq!(written_as(Form::Octets, &tree!("C0")), tree!("c0"));
///
/// // Text is compared as it is written
/// assert_eq!(written_as(Form::Plain, &tree!("C0")), tree!("C0"));
///
/// // A name is the one of its set
/// let causes = &["congestion", "plmn-not-allowed"];
/// assert_eq!(written_as(Form::Named(causes), &tree!("PLMN not allowed")), tree!("plmn-not-allowed"));
/// ```
pub fn written_as(form: Form, written: &Value) -> Value {
    match (form, written) {
        (Form::Octets, Value::String(text)) if unhex(text).is_some() => {
            Value::String(text.to_ascii_lowercase())
        }
        (Form::Named(names), Value::String(text)) => match named(names, text) {
            Some(position) => Value::String(names[position].to_string()),
            None => written.clone(),
        },
        // Each value of a list that someone wrote, as for a comparison with one of several.
        (form, Value::Array(entries)) => Value::Array(
            entries
                .iter()
                .map(|entry| written_as(form, entry))
                .collect(),
        ),
        _ => written.clone(),
    }
}
