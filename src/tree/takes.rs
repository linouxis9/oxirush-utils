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

//! What the tree of a value takes, as a type that reads it from a tree says it.

use alloc::vec::Vec;

use super::same_name;

/// What the tree of a value takes, as the type that reads it from a tree says it: for
/// someone who writes a value, and for a path that is checked against the members of a
/// value without one at hand. A type describes its tree once, as a constant: each member
/// of an object with what it takes, down to its plain values.
///
/// ```
/// use oxirush_utils::tree::Takes;
///
/// // A value of two flags, or of a number written alone as its `value`.
/// const FLAGS: Takes = Takes::Members(None, &[("V4", &Takes::Flag), ("V6", &Takes::Flag)]);
/// const RATE: Takes = Takes::Members(
///     Some("value"),
///     &[("value", &Takes::Number(7)), ("Spare", &Takes::Number(255))],
/// );
/// assert_eq!(FLAGS.member("v4"), Some(&Takes::Flag));
/// assert_eq!(FLAGS.member("V5"), None);
/// assert_eq!(RATE.scalar(), Some("value"));
/// assert_eq!(RATE.names(), ["value", "Spare"]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Takes {
    /// `true` or `false`: a flag.
    Flag,
    /// A number, from 0 to this one.
    Number(u64),
    /// A number from the first to the second.
    Signed(i64, i64),
    /// One of these names, which a reader takes in any case and with any punctuation.
    Named(&'static [&'static str]),
    /// Octets in hexadecimal: this many where they are fixed.
    Octets(Option<usize>),
    /// A text.
    Text,
    /// A name of labels with dots between them, `internet`, as a domain name, an APN or
    /// an FQDN writes it.
    Name,
    /// Digits, as an IMSI, an IMEI or an MSISDN writes them.
    Digits,
    /// An IPv4 address, as text.
    Ipv4,
    /// An IPv6 address, as text.
    Ipv6,
    /// A PLMN, its MCC and its MNC, `208-93`.
    Plmn,
    /// The tree of a message that the value carries, as the tree of a message of its
    /// protocol is written.
    Message,
    /// A list, of entries that each take this.
    List(&'static Takes),
    /// An object of these members, each with what it takes, in their order; a value that
    /// is written as a plain value is the member that the first names, if any.
    Members(
        Option<&'static str>,
        &'static [(&'static str, &'static Takes)],
    ),
    /// One of these forms, the first of them that reads the value.
    OneOf(&'static [&'static Takes]),
}

/// An object of the forms of a [`Takes`]: the member that its plain value is, and its
/// members.
type Object = (
    Option<&'static str>,
    &'static [(&'static str, &'static Takes)],
);

impl Takes {
    /// The first of `found` of the objects of its forms: itself, each form of
    /// [`Takes::OneOf`], and what an entry of a list takes.
    fn find_object<T>(&self, found: &mut impl FnMut(Object) -> Option<T>) -> Option<T> {
        match *self {
            Self::Members(is, members) => found((is, members)),
            Self::OneOf(forms) => forms.iter().find_map(|form| form.find_object(found)),
            Self::List(entry) => entry.find_object(found),
            _ => None,
        }
    }

    /// Whether a form of it is an object: of itself, of [`Takes::OneOf`] or of an entry
    /// of a list.
    pub fn is_object(&self) -> bool {
        self.find_object(&mut |_| Some(())).is_some()
    }

    /// The member that a plain value of it is: that of the first object of its forms that
    /// names one.
    pub fn scalar(&self) -> Option<&'static str> {
        self.find_object(&mut |(is, _)| is)
    }

    /// The members of the objects of its forms, each once, in their order.
    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        self.find_object(&mut |(_, members)| {
            for (name, _) in members {
                if !names.contains(name) {
                    names.push(name);
                }
            }
            None::<()>
        });
        names
    }

    /// What the member `name` of an object of its forms takes, the name in any case and
    /// with any punctuation ([`same_name`]): `None` for a name that none of them has.
    pub fn member(&self, name: &str) -> Option<&'static Takes> {
        self.find_object(&mut |(_, members)| {
            (members.iter())
                .find(|(member, _)| same_name(member, name))
                .map(|(_, takes)| *takes)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The members of every form: of a list's entries, and of each object of a choice,
    /// the first that has a name.
    #[test]
    fn the_members_of_every_form_are_found() {
        const ADDRESS: Takes = Takes::OneOf(&[
            &Takes::Ipv4,
            &Takes::Members(None, &[("FQDN", &Takes::Name)]),
            &Takes::Members(
                Some("Address"),
                &[("Address", &Takes::Octets(None)), ("FQDN", &Takes::Text)],
            ),
        ]);
        assert!(ADDRESS.is_object());
        assert_eq!(ADDRESS.member("fqdn"), Some(&Takes::Name));
        assert_eq!(ADDRESS.member("address"), Some(&Takes::Octets(None)));
        assert_eq!(ADDRESS.scalar(), Some("Address"));
        assert_eq!(ADDRESS.names(), ["FQDN", "Address"]);
        let list = Takes::List(&ADDRESS);
        assert_eq!(list.member("FQDN"), Some(&Takes::Name));
        assert!(!Takes::List(&Takes::Ipv4).is_object());
        assert_eq!(Takes::Flag.member("x"), None);
        assert_eq!(Takes::Flag.names(), Vec::<&str>::new());
    }
}
