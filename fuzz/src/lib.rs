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

//! What the fuzz targets do with the octets that they are given: the programs of
//! `fuzz_targets/` call these functions.
//!
//! `identity` reads the octets, and the text that they are, as each identity. A value
//! that is read writes back what it was read from, a tolerant reader reads what the
//! canonical one reads and more, and what it reads is a value that the canonical one
//! reads from its own octets.
//!
//! `paths` takes a path, a tree and a value out of the octets, and selects, lists and
//! edits: each path that is listed selects its value, an edit that is refused leaves
//! the tree as it was, and a value that is set is the one that is selected.
//!
//! `forms` holds the forms of a tree, the names and the segments of its paths to what
//! they are written to be, and reads any JSON with each reader of `Reading`: what is
//! read writes a tree that reads back as itself, and a refusal of a secret says nothing
//! of it.

use oxirush_utils::identity::{
    Guti5g, GutiEps, Imei, Imeisv, Imsi, ImsiSuci, NaiSuci, Plmn, RoutingIndicator, STmsi5g, Supi,
    Tai5gs, TaiEps, digits, nai, tbcd,
};
use oxirush_utils::tree::Value;
use oxirush_utils::tree::{self, Family, Next, Place, Root, Step};

/// Read `octets`, and the text that they are, as each identity.
pub fn identity(octets: &[u8]) {
    if let Ok(three) = <[u8; 3]>::try_from(octets) {
        if let Ok(plmn) = Plmn::from_nas_octets(three) {
            assert_eq!(plmn.to_nas_octets(), three);
            assert_eq!(plmn.to_string().parse(), Ok(plmn));
        }
        if let Ok(plmn) = Plmn::from_ap_octets(three) {
            assert_eq!(plmn.to_ap_octets(), three);
            assert_eq!(Plmn::new(plmn.mcc(), plmn.mnc()), Ok(plmn));
        }
    }
    if let Ok(text) = tbcd::decode(octets) {
        assert_eq!(tbcd::encode(&text).unwrap(), octets);
    }
    // Digits: the tolerant reader reads what the canonical one reads, and more.
    let strict = digits::decode(octets);
    match digits::decode_tolerant(octets) {
        Ok(((kind, text), faults)) => {
            assert_eq!(strict.is_ok(), faults.is_empty());
            let canonical = digits::encode(kind, &text).unwrap();
            assert_eq!(digits::decode(&canonical).unwrap(), (kind, text));
            assert_eq!(faults.is_empty(), canonical == octets);
        }
        Err(_) => assert!(strict.is_err()),
    }
    for kind in 0..8 {
        if let Ok((imeisv, faults)) = Imeisv::from_nas_octets_tolerant(kind, octets) {
            let canonical = imeisv.to_nas_octets(kind);
            assert_eq!(Imeisv::from_nas_octets(kind, &canonical), Ok(imeisv));
            assert_eq!(
                Imeisv::from_nas_octets(kind, octets).is_ok(),
                faults.is_empty()
            );
        }
        if let Ok(imsi) = Imsi::from_nas_octets(kind, octets) {
            assert_eq!(imsi.to_nas_octets(kind), octets);
        }
        if let Ok(imei) = Imei::from_nas_octets(kind, octets) {
            assert_eq!(imei.to_nas_octets(kind), octets);
        }
    }
    // The temporary identities.
    let strict = Guti5g::from_nas_octets(octets);
    match Guti5g::from_nas_octets_tolerant(octets) {
        Ok((guti, faults)) => {
            assert_eq!(strict.is_ok(), faults.is_empty());
            assert_eq!(Guti5g::from_nas_octets(&guti.to_nas_octets()), Ok(guti));
            assert_eq!(faults.is_empty(), guti.to_nas_octets() == octets);
            assert_eq!(guti.to_string().parse(), Ok(guti));
        }
        Err(_) => assert!(strict.is_err()),
    }
    let strict = STmsi5g::from_nas_octets(octets);
    match STmsi5g::from_nas_octets_tolerant(octets) {
        Ok((s_tmsi, faults)) => {
            assert_eq!(strict.is_ok(), faults.is_empty());
            assert_eq!(STmsi5g::from_u64(s_tmsi.to_u64()), Ok(s_tmsi));
            assert_eq!(faults.is_empty(), s_tmsi.to_nas_octets() == octets);
        }
        Err(_) => assert!(strict.is_err()),
    }
    if let Ok(guti) = GutiEps::from_nas_octets(octets) {
        assert_eq!(guti.to_nas_octets(), octets);
        assert_eq!(guti.to_string().parse(), Ok(guti));
    }
    // The SUCI of an IMSI, and its routing indicator. The value of a tolerant reading is
    // canonical but for a scheme output of another length than its profile gives it,
    // which it keeps for the home network: its octets are read again with that fault
    // alone, and the text of the SUCI, which is read strictly, is refused.
    let strict = ImsiSuci::from_nas_octets(octets);
    match ImsiSuci::from_nas_octets_tolerant(octets) {
        Ok((suci, faults)) => {
            assert_eq!(strict.is_ok(), faults.is_empty());
            let canonical = suci.to_nas_octets();
            let kept = faults.iter().any(|fault| fault.member == "scheme-output");
            let of_the_output = faults.iter().all(|fault| fault.member == "scheme-output");
            assert_eq!(of_the_output, canonical == octets);
            match kept {
                false => {
                    assert_eq!(ImsiSuci::from_nas_octets(&canonical).as_ref(), Ok(&suci));
                    assert_eq!(suci.to_string().parse().as_ref(), Ok(&suci));
                }
                true => {
                    let (again, faults) = ImsiSuci::from_nas_octets_tolerant(&canonical).unwrap();
                    assert_eq!(again, suci);
                    let members: Vec<_> = faults.iter().map(|fault| fault.member).collect();
                    assert_eq!(members, ["scheme-output"]);
                    assert!(suci.to_string().parse::<ImsiSuci>().is_err());
                }
            }
            assert_eq!(suci.msin().is_some(), suci.concealed_output().is_none());
            assert_eq!(suci.msin().is_some(), suci.imsi().is_some());
        }
        Err(_) => assert!(strict.is_err()),
    }
    // The SUCI in the form of a NAI.
    let strict = NaiSuci::from_nas_octets(octets);
    match NaiSuci::from_nas_octets_tolerant(octets) {
        Ok((suci, faults)) => {
            assert_eq!(strict.is_ok(), faults.is_empty());
            let canonical = suci.to_nas_octets().unwrap();
            assert_eq!(NaiSuci::from_nas_octets(&canonical).as_ref(), Ok(&suci));
            assert_eq!(suci.clear().is_some(), suci.concealed_output().is_none());
        }
        Err(_) => assert!(strict.is_err()),
    }
    if let Ok(two) = <[u8; 2]>::try_from(octets)
        && let Ok(indicator) = RoutingIndicator::from_nas_octets(two)
    {
        assert_eq!(indicator.to_nas_octets(), two);
        assert_eq!(indicator.as_str().parse(), Ok(indicator));
    }
    if let Ok(six) = <[u8; 6]>::try_from(octets)
        && let Ok(tai) = Tai5gs::from_nas_octets(six)
    {
        assert_eq!(tai.to_nas_octets(), six);
    }
    if let Ok(five) = <[u8; 5]>::try_from(octets)
        && let Ok(tai) = TaiEps::from_nas_octets(five)
    {
        assert_eq!(tai.to_nas_octets(), five);
    }
    // Text: what is read prints as it was written, but for the case of its digits.
    let Ok(text) = core::str::from_utf8(octets) else {
        return;
    };
    if let Ok(plmn) = text.parse::<Plmn>() {
        assert_eq!(plmn.to_string(), text);
    }
    if let Ok(guti) = text.parse::<Guti5g>() {
        assert_eq!(guti.to_string(), text.to_ascii_lowercase());
    }
    if let Ok(guti) = text.parse::<GutiEps>() {
        assert_eq!(guti.to_string(), text.to_ascii_lowercase());
    }
    for digits in [
        Imsi::new(text).map(|imsi| imsi.to_string()),
        Imei::new(text).map(|imei| imei.to_string()),
    ] {
        if let Ok(digits) = digits {
            assert_eq!(digits, text);
        }
    }
    if let Ok(imei) = Imei::with_check_digit(text) {
        assert!(imei.has_check_digit());
    }
    if let Ok(supi) = Supi::parse(text) {
        assert_eq!(supi.to_string(), text);
        assert_eq!(text.parse(), Ok(supi.clone()));
        assert_eq!(supi.as_imsi().is_some(), text.starts_with("imsi-"));
    }
    if let Ok(suci) = text.parse::<ImsiSuci>() {
        assert_eq!(suci.to_string(), text.to_ascii_lowercase());
        assert_eq!(ImsiSuci::from_nas_octets(&suci.to_nas_octets()), Ok(suci));
    }
    // A SUCI in the form of a NAI writes its octets in lower case, and the rest as it came.
    if let Ok(suci) = text.parse::<NaiSuci>() {
        assert_eq!(suci.to_string().parse().as_ref(), Ok(&suci));
        if !text.bytes().any(|c| c.is_ascii_uppercase()) {
            assert_eq!(suci.to_string(), text);
        }
        if let Ok(octets) = suci.to_nas_octets() {
            assert_eq!(NaiSuci::from_nas_octets(&octets).as_ref(), Ok(&suci));
        }
        assert_eq!(nai::check_realm(suci.realm()), Ok(()));
    }
    if let Some((username, realm)) = text.rsplit_once('@')
        && nai::check_realm(realm).is_ok()
        && nai::check_username(username).is_ok()
    {
        assert!(!username.is_empty() && !realm.is_empty());
    }
}

/// A family whose paths have a root, and in whose lists an entry is selected by its
/// `id` too.
struct ById;

impl Family for ById {
    type Shape = ();

    fn root(&self) -> Root<'_> {
        Root::new("t", &[], true)
    }

    fn step<'v>(
        &self,
        place: &Place<'v, ()>,
        segment: &str,
        next: &mut Next<'v, ()>,
    ) -> Result<Step<()>, tree::Error> {
        let Some(entries) = place.value().and_then(Value::as_array) else {
            return Ok(Step::Plain);
        };
        let named = |entry: &Value| entry.get("id").and_then(Value::as_str) == Some(segment);
        if !entries.iter().any(named) {
            return Ok(Step::Plain);
        }
        for (position, entry) in entries.iter().enumerate().filter(|(_, entry)| named(entry)) {
            next.entry(position, entry, ());
        }
        Ok(Step::Taken)
    }
}

/// Whether a member of `tree` is named `*`, at any depth: a path names no such member.
fn has_a_star(tree: &Value) -> bool {
    match tree {
        Value::Object(members) => members
            .iter()
            .any(|(name, member)| name == "*" || has_a_star(member)),
        Value::Array(entries) => entries.iter().any(has_a_star),
        _ => false,
    }
}

/// Take a path, a tree and a value out of `octets`, which a zero octet separates, and
/// select, list and edit.
pub fn paths(octets: &[u8]) {
    let mut parts = octets.splitn(3, |octet| *octet == 0);
    let (Some(path), Some(tree), value) = (parts.next(), parts.next(), parts.next()) else {
        return;
    };
    let path = String::from_utf8_lossy(path);
    let Ok(tree) = serde_json::from_slice::<Value>(tree) else {
        return;
    };
    let value = value.and_then(|value| serde_json::from_slice::<Value>(value).ok());
    let value = value.unwrap_or(Value::Bool(true));

    // Each path that is listed selects its value, in a tree without a member `*`.
    let plain = !has_a_star(&tree);
    for (listed, held) in tree::paths(&tree) {
        if plain && listed.len() <= 4096 {
            let selected = tree::select(&tree, &listed).unwrap();
            assert!(selected.contains(&&held), "{listed}");
        }
    }
    for (listed, _) in tree::paths_in(&ById, &tree) {
        assert!(listed.starts_with("/t"), "{listed}");
    }
    // Any path selects, or is refused: no more values than a path selects.
    if let Ok(selected) = tree::select(&tree, &path) {
        assert!(selected.len() <= 65_536);
        assert!(selected.iter().all(|value| !value.is_null()));
    }
    let _ = tree::select_in(&ById, &tree, &path);

    // An edit is done whole, or not at all.
    let mut edited = tree.clone();
    match tree::set(&mut edited, &path, value.clone()) {
        // What was set is what is selected, where the value is one.
        Ok(()) if !value.is_null() && plain => {
            let selected = tree::select(&edited, &path).unwrap();
            assert!(
                !selected.is_empty() && selected.iter().all(|held| **held == value),
                "{path}"
            );
        }
        Ok(()) => {}
        Err(_) => assert_eq!(edited, tree),
    }
    let mut edited = tree.clone();
    if tree::remove(&mut edited, &path).is_err() {
        assert_eq!(edited, tree);
    }
    let mut edited = tree.clone();
    if tree::insert(&mut edited, &path, value.clone()).is_err() {
        assert_eq!(edited, tree);
    }
    for edit in 0..3 {
        let mut edited = tree.clone();
        let done = match edit {
            0 => tree::set_in(&ById, &mut edited, &path, value.clone()),
            1 => tree::remove_in(&ById, &mut edited, &path),
            _ => tree::insert_in(&ById, &mut edited, &path, value.clone()),
        };
        if done.is_err() {
            assert_eq!(edited, tree);
        }
    }
}

/// A type with a member of each reader of [`tree::Reading`], which the target `forms`
/// reads from any JSON.
#[derive(Debug, PartialEq)]
struct Every {
    flag: bool,
    nothing: Option<()>,
    text: String,
    choice: usize,
    small: u64,
    signed: i64,
    octets: Vec<u8>,
    array: Option<[u8; 3]>,
    bits: Option<(Vec<u8>, usize)>,
    list: Vec<u64>,
    alternative: (usize, u64),
    key: Option<Vec<u8>>,
}

/// A string of bits, as [`tree::Reading::bits`] reads one.
struct Bits(Vec<u8>, usize);

impl tree::Tree for Bits {
    const NAME: &'static str = "Bits";

    fn to_tree(&self) -> Value {
        tree::bits(&self.0, self.1)
    }

    fn from_tree(value: &Value, reading: &mut tree::Reading) -> Result<Self, tree::Refused> {
        let (octets, length) = reading.bits(value)?;
        Ok(Self(octets, length))
    }
}

const CHOICES: &[&str] = &["first", "second-one", "3rd"];
const ALTERNATIVES: &[&str] = &["a-number", "another"];
const MEMBERS: &[&str] = &[
    "flag",
    "nothing",
    "text",
    "choice",
    "small",
    "signed",
    "octets",
    "array",
    "bits",
    "list",
    "alternative",
    "key",
];

impl tree::Tree for Every {
    const NAME: &'static str = "Every";

    fn to_tree(&self) -> Value {
        let mut members = tree::Members::new();
        members.put("flag", Value::Bool(self.flag));
        members.put_optional("nothing", self.nothing.map(|()| Value::Null));
        members.put("text", tree::text(&self.text));
        members.put("choice", tree::text(CHOICES[self.choice]));
        members.put("small", tree::unsigned(self.small));
        members.put("signed", tree::signed(self.signed));
        members.put("octets", tree::octets(&self.octets));
        members.put_optional("array", self.array.map(|array| tree::octets(&array)));
        members.put_optional(
            "bits",
            self.bits
                .as_ref()
                .map(|(octets, length)| tree::bits(octets, *length)),
        );
        members.put(
            "list",
            Value::Array(
                self.list
                    .iter()
                    .map(|entry| tree::unsigned(*entry))
                    .collect(),
            ),
        );
        let (alternative, inner) = self.alternative;
        members.put(
            "alternative",
            tree::alternative(ALTERNATIVES[alternative], tree::unsigned(inner)),
        );
        members.put_optional("key", self.key.as_ref().map(|key| tree::octets(key)));
        members.finish()
    }

    fn from_tree(value: &Value, reading: &mut tree::Reading) -> Result<Self, tree::Refused> {
        let read = reading.members(value, MEMBERS)?;
        Ok(Self {
            flag: reading.member(&read, "flag", |value, reading| reading.boolean(value))?,
            nothing: reading.optional(&read, "nothing", |value, reading| reading.null(value))?,
            text: reading.member(&read, "text", |value, reading| {
                reading.text(value).map(str::to_owned)
            })?,
            choice: reading.member(&read, "choice", |value, reading| {
                reading.named(value, CHOICES)
            })?,
            small: reading.member(&read, "small", |value, reading| {
                reading.unsigned(value, 3, 300)
            })?,
            signed: reading.member(&read, "signed", |value, reading| {
                reading.signed(value, -5, 5)
            })?,
            octets: reading.member(&read, "octets", |value, reading| reading.octets(value))?,
            array: reading.optional(&read, "array", |value, reading| reading.octet_array(value))?,
            bits: reading.optional(&read, "bits", |value, reading| reading.bits(value))?,
            list: reading.member(&read, "list", |value, reading| {
                reading.list(value, |entry, reading| reading.unsigned(entry, 0, u64::MAX))
            })?,
            alternative: reading.member(&read, "alternative", |value, reading| {
                let (alternative, inner) = reading.alternative(value, ALTERNATIVES)?;
                let inner =
                    reading.within(ALTERNATIVES[alternative], inner, |inner, reading| {
                        reading.unsigned(inner, 0, 9)
                    })?;
                Ok((alternative, inner))
            })?,
            key: reading.optional(&read, "key", |value, reading| {
                reading.secret(|reading| reading.octets(value))
            })?,
        })
    }
}

/// The forms of a tree, the names and the segments of its paths, on the text that the
/// octets start with, up to a zero octet, and its readers on the JSON after it.
///
/// Hexadecimal reads back what it writes, and reads what is two digits for each octet
/// alone, which it writes back in lower case. A segment that [`tree::escape`] writes is
/// one that [`tree::segments`] reads back, alone and among others, and a position is a
/// number written plainly. [`tree::same_name`] is symmetric, and [`tree::named`] finds
/// the name itself, then one written alike but for its case and its separators, then the
/// first of the same name. Bits written by [`tree::bits`] read back.
///
/// A type with a member of each reader reads any JSON without a panic; a value that it
/// reads writes a tree that reads back as itself, and whose own tree is that tree. A
/// refusal of a secret member says nothing of what is written there.
pub fn forms(octets: &[u8]) {
    let (text, json) = match octets.iter().position(|octet| *octet == 0) {
        Some(at) => (&octets[..at], &octets[at + 1..]),
        None => (octets, &[][..]),
    };
    let text = String::from_utf8_lossy(text);

    // Hexadecimal
    assert_eq!(
        tree::unhex(&tree::hex(text.as_bytes())).as_deref(),
        Some(text.as_bytes())
    );
    match tree::unhex(&text) {
        Some(read) => {
            assert_eq!(tree::hex(&read), text.to_ascii_lowercase(), "{text:?}");
        }
        None => assert!(
            text.len() % 2 == 1 || !text.bytes().all(|digit| digit.is_ascii_hexdigit()),
            "{text:?}"
        ),
    }

    // Segments, escapes and positions
    let pieces: Vec<&str> = text.split('\u{1}').take(8).collect();
    let path: String = pieces
        .iter()
        .map(|piece| format!("/{}", tree::escape(piece)))
        .collect();
    if path.len() <= 4096 {
        let read = tree::segments(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
        assert_eq!(read, pieces, "{path:?}");
    }
    let plain = text == "0"
        || (!text.is_empty()
            && !text.starts_with('0')
            && text.bytes().all(|digit| digit.is_ascii_digit()));
    match tree::position(&text) {
        Some(number) => assert!(plain && number.to_string() == *text, "{text:?}"),
        None => assert!(!plain || text.parse::<usize>().is_err(), "{text:?}"),
    }

    // Names
    let names: Vec<&str> = pieces.iter().copied().skip(1).collect();
    if let Some(name) = pieces.first() {
        for other in &names {
            assert_eq!(
                tree::same_name(name, other),
                tree::same_name(other, name),
                "{name:?} {other:?}"
            );
        }
        // The name itself wins, then one written alike but for its case and its
        // separators, then the first of the same name
        let separator = |c: u8| matches!(c, b'-' | b'_' | b' ');
        let alike = |a: &str, b: &str| {
            a.len() == b.len()
                && (a.bytes().zip(b.bytes()))
                    .all(|(a, b)| a.eq_ignore_ascii_case(&b) || (separator(a) && separator(b)))
        };
        let first = |is: &dyn Fn(&str) -> bool| names.iter().position(|known| is(known));
        let expected = first(&|known| known == *name)
            .or_else(|| first(&|known| alike(known, name)))
            .or_else(|| first(&|known| tree::same_name(known, name)));
        assert_eq!(tree::named(&names, name), expected, "{name:?} {names:?}");
    }

    // Bits: the octets of the text with the bits after the length cleared
    let bytes = text.as_bytes();
    if !bytes.is_empty() {
        let length = 8 * bytes.len() - usize::from(bytes[0] % 8);
        let mut cleared = bytes.to_vec();
        if !length.is_multiple_of(8) {
            *cleared.last_mut().expect("an octet") &= 0xffu8 << (8 - length % 8);
        }
        let read = tree::from_value::<Bits>(&tree::bits(&cleared, length));
        assert!(
            read.as_ref()
                .is_ok_and(|Bits(octets, bits)| *octets == cleared && *bits == length),
            "{cleared:02x?} {length}"
        );
    }

    // The readers on any JSON
    let Ok(value) = serde_json::from_slice::<Value>(json) else {
        return;
    };
    match tree::from_value::<Every>(&value) {
        Ok(every) => {
            let written = tree::Tree::to_tree(&every);
            let again: Every =
                tree::from_value(&written).unwrap_or_else(|error| panic!("{written}: {error}"));
            assert_eq!(again, every, "{written}");
            assert_eq!(tree::Tree::to_tree(&again), written);
        }
        Err(error) => {
            let said = error.to_string();
            if let Some(Value::String(key)) = value.get("key")
                && key.len() >= 6
                && error.at().contains("key")
            {
                assert!(
                    !said.contains(key.as_str()),
                    "a refusal of a secret says it: {said}"
                );
            }
        }
    }
}
