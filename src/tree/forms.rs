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
use alloc::borrow::Cow;
use alloc::string::String;
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
    let (pairs, rest) = text.as_bytes().as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    let mut octets = vec![0; pairs.len()];
    // Eight digits at a time in a u64, then the last pairs one by one.
    let (eights, _) = text.as_bytes().as_chunks::<8>();
    let (fours, _) = octets.as_chunks_mut::<4>();
    let mut valid = true;
    for (four, eight) in fours.iter_mut().zip(eights) {
        let (octets, digits) = four_octets(u64::from_le_bytes(*eight));
        valid &= digits;
        *four = octets.to_le_bytes();
    }
    let done = eights.len() * 4;
    let mut other = 0;
    for (octet, [high, low]) in octets[done..].iter_mut().zip(&pairs[done..]) {
        let (high, low) = (digit(*high), digit(*low));
        other |= high | low;
        *octet = high << 4 | low & 15;
    }
    (valid && other & NO_DIGIT == 0).then_some(octets)
}

/// What [`digit`] gives for a character that is no digit: a value that no half octet
/// has.
const NO_DIGIT: u8 = 0x80;

/// The value of a hexadecimal digit, or [`NO_DIGIT`], without a branch that leaves the
/// loop. A letter is taken in lower case, which differs from upper case by one bit.
fn digit(digit: u8) -> u8 {
    let (number, letter) = (digit.wrapping_sub(b'0'), (digit | 0x20).wrapping_sub(b'a'));
    match (number < 10, letter < 6) {
        (true, _) => number,
        (false, true) => letter + 10,
        (false, false) => NO_DIGIT,
    }
}

/// The four octets that eight characters, the octets of a `u64` in little-endian
/// order, write in hexadecimal, and whether all eight are digits. Each byte is
/// compared at once as a lane of the `u64`: with its high bit set, a lane minus a
/// number below 0x80 keeps that bit when the lane was at least the number, and borrows
/// nothing from the next lane.
fn four_octets(characters: u64) -> (u32, bool) {
    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGH: u64 = ONES * 0x80;
    const EVEN: u64 = 0x00ff_00ff_00ff_00ff;
    const CASE: u64 = ONES * 0x20;
    const HALF: u64 = ONES * 0x0f;
    const NINE: u64 = ONES * 9;
    let at_least = |lanes: u64, lowest: u8| ((lanes | HIGH) - u64::from(lowest) * ONES) & HIGH;
    let lower = characters | CASE;
    let number = at_least(characters, b'0') & !at_least(characters, b'9' + 1);
    let letter = at_least(lower, b'a') & !at_least(lower, b'f' + 1);
    // A character outside ASCII has its high bit set and is no digit.
    let digits = characters & HIGH == 0 && (number | letter) == HIGH;
    // The value of a digit is its low four bits, and of a letter those plus 9; the high
    // bit of a lane becomes a mask of the whole lane.
    let (number, letter) = ((number >> 7) * 0xff, (letter >> 7) * 0xff);
    let halves = (characters & HALF & number) | ((lower & HALF) + NINE) & letter;
    // The first character of each pair is the high half of its octet.
    let pairs = (halves & EVEN) << 4 | (halves >> 8) & EVEN;
    // One octet in each 16-bit lane, moved next to each other.
    let pairs = (pairs | pairs >> 8) & 0x0000_ffff_0000_ffff;
    ((pairs | pairs >> 16) as u32, digits)
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

/// The names of `names` that are nearest to `name`, which none of them is: `most` of them
/// at most, the nearest first, and none that differs from it in more than a third of its
/// letters and its digits (two for a short name). For the error of a name that is
/// misspelled, which says what it may be.
///
/// ```
/// use oxirush_utils::tree::nearest;
///
/// let names = ["AMF-UE-NGAP-ID", "RAN-UE-NGAP-ID", "NAS-PDU", "Cause"];
/// assert_eq!(nearest(&names, "AMF-UE-NGAP-IDD", 3), ["AMF-UE-NGAP-ID", "RAN-UE-NGAP-ID"]);
/// assert!(nearest(&names, "TimeToWait", 3).is_empty());
/// ```
pub fn nearest<'n, T: AsRef<str>>(names: &'n [T], name: &str, most: usize) -> Vec<&'n str> {
    let written: Vec<u8> = letters(name).collect();
    let farthest = (written.len() / 3).max(2);
    let mut near: Vec<(usize, usize, &str)> = names
        .iter()
        .enumerate()
        .filter_map(|(position, known)| {
            let known = known.as_ref();
            let distance = distance(&written, &letters(known).collect::<Vec<_>>());
            (distance <= farthest).then_some((distance, position, known))
        })
        .collect();
    near.sort_unstable();
    near.into_iter().take(most).map(|(.., name)| name).collect()
}

/// The edits of one letter that make `a` into `b` (Levenshtein).
fn distance(a: &[u8], b: &[u8]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = match x == y {
                true => diagonal,
                false => 1 + diagonal.min(above).min(row[j]),
            };
            diagonal = above;
        }
    }
    row[b.len()]
}

/// The name of `members` that `name` is, as [`named`] finds it.
pub(crate) fn key<'m>(members: &'m Map, name: &str) -> Option<&'m str> {
    // The name as it is written is the usual case, and is found without a search.
    if let Some((known, _)) = members.get_key_value(name) {
        return Some(known);
    }
    let alike = members.keys().find(|known| alike(known, name));
    alike.or_else(|| members.keys().find(|known| same_name(known, name)))
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
    Value::String(hex(octets).into())
}

/// The form of text: a `String`, or a `&'static str`, as the name of a value of an
/// enumeration, which is not copied.
pub fn text(text: impl Into<Cow<'static, str>>) -> Value {
    Value::String(text.into())
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
    members.push("value".into(), Value::String(hex(octets).into()));
    members.push("length".into(), Value::Number(Number::from(length)));
    Value::Object(members)
}

/// The form of a value that is one of several alternatives, with content: an object with
/// one member, the name of the alternative with its value. An alternative without
/// content is its name alone, as [`text`].
pub fn alternative(name: impl Into<Cow<'static, str>>, value: Value) -> Value {
    let mut members = Map::new();
    members.push(name.into(), value);
    Value::Object(members)
}
