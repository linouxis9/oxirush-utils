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

//! Why text or octets are not an identity.

use core::fmt;

/// Why a value is not the identity that it was to be: the member at fault, and what is
/// wrong with it.
///
/// ```
/// use oxirush_utils::identity::{Fault, Plmn};
///
/// let error = Plmn::new("208", "9").unwrap_err();
/// assert_eq!(error.member, "mnc");
/// assert!(matches!(error.fault, Fault::Length { given: 1, min: 2, max: 3, .. }));
/// assert_eq!(error.to_string(), "`mnc` has 1 digit, and has 2 or 3");
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    /// The member at fault, by the name that the specification gives it, in lower case
    /// with hyphens: `mnc`, `amf-set-id`.
    pub member: &'static str,
    /// What is wrong with it.
    pub fault: Fault,
}

/// What is wrong with a member of an identity.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Too few or too many digits, octets or characters.
    Length {
        /// How many were given.
        given: usize,
        /// The fewest that the member has.
        min: usize,
        /// The most that it has.
        max: usize,
        /// What is counted: `digits`, `octets`.
        unit: &'static str,
    },
    /// A number above the largest that the member holds.
    Range {
        /// The number given.
        given: u64,
        /// The largest that the member holds.
        max: u64,
    },
    /// A number that is none of those that the member has.
    Value {
        /// The number given.
        given: u64,
        /// What the member is.
        expected: &'static str,
    },
    /// A character that the member does not have.
    Character {
        /// The character given.
        given: char,
        /// What the member has: `decimal digits`.
        expected: &'static str,
    },
    /// A half octet that is no decimal digit, where the coding has a digit.
    Digit {
        /// The half octet.
        given: u8,
    },
    /// A half octet that is not the filler or the end mark `1111`, where the coding has
    /// it.
    EndMark {
        /// The half octet.
        given: u8,
    },
    /// An octet whose fixed bits are not those of the coding.
    Octet {
        /// The octet given.
        given: u8,
        /// The octet that the coding has there.
        expected: u8,
    },
    /// A spare bit that is 1, where the sender writes 0.
    Spare {
        /// The octet with the spare bit.
        given: u8,
    },
    /// Text that is not written in the form of the identity.
    Form {
        /// The form, with an example.
        expected: &'static str,
    },
}

impl Error {
    pub(crate) const fn new(member: &'static str, fault: Fault) -> Self {
        Self { member, fault }
    }

    pub(crate) const fn length(
        member: &'static str,
        unit: &'static str,
        given: usize,
        min: usize,
        max: usize,
    ) -> Self {
        let fault = Fault::Length {
            given,
            min,
            max,
            unit,
        };
        Self::new(member, fault)
    }

    /// The same fault, of another member: that of the identity which holds the first.
    pub(crate) const fn of(mut self, member: &'static str) -> Self {
        self.member = member;
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let member = self.member;
        match self.fault {
            Fault::Length {
                given,
                min,
                max,
                unit,
            } => {
                // `digits` for any count but one.
                let one = unit.strip_suffix('s').unwrap_or(unit);
                let counted = if given == 1 { one } else { unit };
                write!(f, "`{member}` has {given} {counted}, and has ")?;
                match (min, max) {
                    (min, usize::MAX) => write!(f, "{min} at least"),
                    (min, max) if min == max => write!(f, "{min}"),
                    (min, max) if min + 1 == max => write!(f, "{min} or {max}"),
                    (min, max) => write!(f, "{min} to {max}"),
                }
            }
            Fault::Range { given, max } => {
                write!(f, "`{member}` is {given}, and is {max} at most")
            }
            Fault::Value { given, expected } => {
                write!(f, "`{member}` is {given}, and is {expected}")
            }
            Fault::Character { given, expected } => {
                write!(
                    f,
                    "`{member}` has the character `{given}`, and has {expected}"
                )
            }
            Fault::Digit { given } => {
                write!(
                    f,
                    "`{member}` has the half octet {given:#x} where a digit is, and a digit is 0 to 9"
                )
            }
            Fault::EndMark { given } => {
                write!(
                    f,
                    "`{member}` has the half octet {given:#x} where its end mark is, and the end mark is 0xf"
                )
            }
            Fault::Octet { given, expected } => {
                write!(
                    f,
                    "`{member}` has the octet {given:#04x}, and has {expected:#04x} there"
                )
            }
            Fault::Spare { given } => {
                write!(
                    f,
                    "`{member}` has a spare bit set in the octet {given:#04x}, and a spare bit is 0"
                )
            }
            Fault::Form { expected } => write!(f, "`{member}` is written as {expected}"),
        }
    }
}

impl core::error::Error for Error {}
