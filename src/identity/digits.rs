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

//! Digits in half octets: TBCD strings, the identity digits of a mobile identity, and the
//! IMSI, the IMEI and the IMEISV.

use super::plmn::FILLER;
use super::{Error, Fault, Faults, Plmn, Tolerated};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

/// TBCD strings: the `TBCD-STRING` of 3GPP TS 29.002, two characters in an octet, the
/// first in its low half.
///
/// The half octets 0 to 9 are the digits, and 10 to 14 are `*`, `#`, `a`, `b` and `c`.
/// A string of an odd number of characters ends with the filler `1111` in the high half
/// of its last octet, which is the only place of a filler.
///
/// ```
/// use oxirush_utils::identity::tbcd;
///
/// assert_eq!(tbcd::encode("12345")?, [0x21, 0x43, 0xf5]);
/// assert_eq!(tbcd::decode(&[0x21, 0x43, 0xf5])?, "12345");
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
pub mod tbcd {
    use super::*;

    /// The characters of a TBCD string, by the value of their half octet.
    const CHARACTERS: &[u8; 15] = b"0123456789*#abc";

    /// The octets of `text` as a TBCD string.
    ///
    /// # Errors
    ///
    /// A character is none of `0` to `9`, `*`, `#`, `a`, `b` and `c`.
    pub fn encode(text: &str) -> Result<Vec<u8>, Error> {
        let half = |character: char| {
            let known = CHARACTERS
                .iter()
                .position(|known| char::from(*known) == character);
            let expected = "digits, *, #, a, b and c";
            let fault = Fault::Character {
                given: character,
                expected,
            };
            known
                .map(|half| half as u8)
                .ok_or(Error::new("tbcd", fault))
        };
        let mut octets = Vec::with_capacity(text.len().div_ceil(2));
        let mut characters = text.chars();
        while let Some(first) = characters.next() {
            let second = match characters.next() {
                Some(second) => half(second)?,
                None => FILLER,
            };
            octets.push(second << 4 | half(first)?);
        }
        Ok(octets)
    }

    /// The text that `octets` are as a TBCD string.
    ///
    /// # Errors
    ///
    /// A filler stands anywhere but in the high half of the last octet.
    pub fn decode(octets: &[u8]) -> Result<String, Error> {
        let mut text = String::with_capacity(2 * octets.len());
        for (index, octet) in octets.iter().enumerate() {
            let (low, high) = (octet & 0x0f, octet >> 4);
            let last = index + 1 == octets.len();
            if low == FILLER || (high == FILLER && !last) {
                return Err(Error::new("tbcd", Fault::Digit { given: FILLER }));
            }
            text.push(char::from(CHARACTERS[usize::from(low)]));
            if high != FILLER {
                text.push(char::from(CHARACTERS[usize::from(high)]));
            }
        }
        Ok(text)
    }
}

/// The octets of the identity digits of a mobile identity: 3GPP TS 24.008 clause
/// 10.5.1.4, which TS 24.301 clause 9.9.3.12 and TS 24.501 clause 9.11.3.4 follow for
/// the IMSI, the IMEI and the IMEISV.
///
/// The first octet has the first digit in its high half, the odd/even indication in bit
/// 4 and the type of identity in bits 3 to 1. Each octet after it has two digits, the
/// first in its low half. With an even number of digits, the high half of the last octet
/// is the end mark `1111`.
///
/// # Errors
///
/// `type_of_identity` is above 7, or `digits` are no decimal digits, or there is none.
///
/// ```
/// use oxirush_utils::identity::digits;
///
/// // An IMSI of 15 digits, type of identity 1: odd
/// let octets = digits::encode(1, "208930000000001")?;
/// assert_eq!(octets, [0x29, 0x80, 0x39, 0x00, 0x00, 0x00, 0x00, 0x10]);
/// assert_eq!(digits::decode(&octets)?, (1, "208930000000001".to_string()));
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
pub fn encode(type_of_identity: u8, digits: &str) -> Result<Vec<u8>, Error> {
    if type_of_identity > 7 {
        let fault = Fault::Range {
            given: type_of_identity.into(),
            max: 7,
        };
        return Err(Error::new("type-of-identity", fault));
    }
    decimal("identity-digits", digits, 1, usize::MAX)?;
    let digits = digits.as_bytes();
    let odd = (digits.len() as u8 & 1) << 3;
    let mut octets = Vec::with_capacity(1 + digits.len() / 2);
    octets.push((digits[0] - b'0') << 4 | odd | type_of_identity);
    for pair in digits[1..].chunks(2) {
        let high = pair.get(1).map_or(FILLER, |digit| digit - b'0');
        octets.push(high << 4 | (pair[0] - b'0'));
    }
    Ok(octets)
}

/// The type of identity and the digits that `octets` hold, as [`encode`] writes them.
///
/// # Errors
///
/// There is no octet, a half octet is no decimal digit where a digit is, the odd/even
/// indication says an even number of digits and the octets have one digit, or the end
/// mark of an even number of digits is not `1111`.
pub fn decode(octets: &[u8]) -> Result<(u8, String), Error> {
    let (read, faults) = decode_tolerant(octets)?;
    match faults.first() {
        Some(fault) => Err(*fault),
        None => Ok(read),
    }
}

/// [`decode`], for octets as other implementations send them: the value, and each thing
/// that a sender by the clause would have written otherwise.
///
/// One departure is read. With an even number of digits, the high half of the last
/// octet is taken as the end mark whatever it holds, and is a fault when it is not
/// `1111`: TS 24.008 clause 10.5.1.4 has it "filled with an end mark coded as 1111", the
/// odd/even indication alone says how many digits there are, and PacketRusher 20250225
/// sends an IMEISV with `0000` there, which Open5GS 2.8.0, free5GC 4.2.3 and pycrate
/// 0.8.1 read.
///
/// # Errors
///
/// What [`decode`] refuses but for that end mark: octets that are not the digits of an
/// identity however they are read.
pub fn decode_tolerant(octets: &[u8]) -> Result<Tolerated<(u8, String)>, Error> {
    let member = "identity-digits";
    let Some((first, rest)) = octets.split_first() else {
        return Err(Error::length(member, "octets", 0, 1, usize::MAX));
    };
    let digit = |half: u8| match half {
        0..=9 => Ok(char::from(b'0' + half)),
        given => Err(Error::new(member, Fault::Digit { given })),
    };
    let (odd, kind) = (first & 0x08 != 0, first & 0x07);
    let mut digits = String::with_capacity(octets.len() * 2);
    let mut faults = Faults::new();
    digits.push(digit(first >> 4)?);
    // An even number of digits has two octets at least: the first has one digit.
    if !odd && rest.is_empty() {
        return Err(Error::length(member, "octets", 1, 2, usize::MAX));
    }
    for (index, octet) in rest.iter().enumerate() {
        let (low, high) = (octet & 0x0f, octet >> 4);
        digits.push(digit(low)?);
        if odd || index + 1 < rest.len() {
            digits.push(digit(high)?);
        } else if high != FILLER {
            faults.push(Error::new(member, Fault::EndMark { given: high }));
        }
    }
    Ok(((kind, digits), faults))
}

/// Check that `text` is `min` to `max` decimal digits.
fn decimal(member: &'static str, text: &str, min: usize, max: usize) -> Result<(), Error> {
    if !(min..=max).contains(&text.len()) {
        return Err(Error::length(member, "digits", text.len(), min, max));
    }
    match text.chars().find(|character| !character.is_ascii_digit()) {
        Some(given) => {
            let expected = "decimal digits";
            Err(Error::new(member, Fault::Character { given, expected }))
        }
        None => Ok(()),
    }
}

/// Up to `N` decimal digits, as ASCII: a value that is copied.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Decimal<const N: usize> {
    /// The digits, then `0` up to `N`.
    digits: [u8; N],
    length: u8,
}

impl<const N: usize> Decimal<N> {
    fn new(member: &'static str, text: &str, min: usize) -> Result<Self, Error> {
        decimal(member, text, min, N)?;
        let mut digits = [b'0'; N];
        digits[..text.len()].copy_from_slice(text.as_bytes());
        Ok(Self {
            digits,
            length: text.len() as u8,
        })
    }

    fn as_str(&self) -> &str {
        // The digits are ASCII.
        core::str::from_utf8(&self.digits[..usize::from(self.length)]).unwrap_or_default()
    }
}

/// The identity of `kind` that `octets` hold, strictly or with the faults of a tolerant
/// reading.
fn read<T>(
    member: &'static str,
    type_of_identity: u8,
    octets: &[u8],
    new: impl FnOnce(&str) -> Result<T, Error>,
) -> Result<Tolerated<T>, Error> {
    let ((kind, digits), faults) = decode_tolerant(octets).map_err(|error| error.of(member))?;
    if kind != type_of_identity & 0x07 {
        let fault = Fault::Octet {
            given: octets[0],
            expected: octets[0] & 0xf8 | type_of_identity & 0x07,
        };
        return Err(Error::new(member, fault));
    }
    Ok((new(&digits)?, faults.of(member)))
}

/// The value of a tolerant reading when it has no fault, and its first fault otherwise.
fn strict<T>(read: Tolerated<T>) -> Result<T, Error> {
    match read.1.first() {
        Some(fault) => Err(*fault),
        None => Ok(read.0),
    }
}

macro_rules! digits_identity {
    ($name:ident, $member:literal, $min:literal, $max:literal, $what:literal) => {
        impl $name {
            #[doc = concat!("The digits of the ", $what, ".")]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            #[doc = concat!("The octets of the ", $what, " as the identity digits of a mobile")]
            /// identity ([`digits::encode`](encode)), with the type of identity that the
            /// information element gives it: bits 3 to 1 of the first octet.
            ///
            /// # Errors
            ///
            /// `type_of_identity` is above 7.
            pub fn to_nas_octets(&self, type_of_identity: u8) -> Result<Vec<u8>, Error> {
                encode(type_of_identity, self.as_str())
            }

            #[doc = concat!("The ", $what, " that `octets` hold as the identity digits of a")]
            /// mobile identity of the type `type_of_identity`.
            ///
            /// # Errors
            ///
            /// What [`digits::decode`](decode) refuses, another type of identity, or
            /// another number of digits.
            pub fn from_nas_octets(type_of_identity: u8, octets: &[u8]) -> Result<Self, Error> {
                strict(Self::from_nas_octets_tolerant(type_of_identity, octets)?)
            }

            /// [`from_nas_octets`](Self::from_nas_octets), for octets as other
            /// implementations send them: the value, and what a sender by the clause
            /// would have written otherwise, which is what
            /// [`digits::decode_tolerant`](decode_tolerant) reads.
            ///
            /// # Errors
            ///
            #[doc = concat!("Octets that are no ", $what, " however they are read.")]
            pub fn from_nas_octets_tolerant(
                type_of_identity: u8,
                octets: &[u8],
            ) -> Result<Tolerated<Self>, Error> {
                read($member, type_of_identity, octets, Self::new)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(text: &str) -> Result<Self, Error> {
                Self::new(text)
            }
        }
    };
}

/// An IMSI: an MCC, an MNC of two or three digits and an MSIN, 15 decimal digits at most
/// (3GPP TS 23.003 clauses 2.2 and 2.3), and so six at least.
///
/// The digits do not say how long the MNC is: [`plmn`](Self::plmn) and
/// [`msin`](Self::msin) are told.
///
/// ```
/// use oxirush_utils::identity::Imsi;
///
/// let imsi = Imsi::new("208930000000001")?;
/// assert_eq!(imsi.plmn(2)?.to_string(), "208-93");
/// assert_eq!(imsi.msin(2)?, "0000000001");
///
/// // In an EPS mobile identity, where the type of identity of an IMSI is 1
/// assert_eq!(imsi.to_nas_octets(1)?, [0x29, 0x80, 0x39, 0x00, 0x00, 0x00, 0x00, 0x10]);
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Imsi(Decimal<15>);

impl Imsi {
    /// The IMSI of its digits.
    ///
    /// # Errors
    ///
    /// `digits` are not 6 to 15 decimal digits.
    pub fn new(digits: &str) -> Result<Self, Error> {
        Decimal::new("imsi", digits, 6).map(Self)
    }

    /// The PLMN of the IMSI, for an MNC of `mnc_digits` digits.
    ///
    /// # Errors
    ///
    /// `mnc_digits` is neither 2 nor 3.
    pub fn plmn(&self, mnc_digits: usize) -> Result<Plmn, Error> {
        let digits = self.as_str();
        let mnc = digits.get(3..3 + mnc_digits).unwrap_or_default();
        Plmn::new(&digits[..3], mnc)
    }

    /// The MSIN of the IMSI, for an MNC of `mnc_digits` digits: the digits after the MCC
    /// and the MNC.
    ///
    /// # Errors
    ///
    /// `mnc_digits` is neither 2 nor 3.
    pub fn msin(&self, mnc_digits: usize) -> Result<&str, Error> {
        self.plmn(mnc_digits)?;
        Ok(&self.as_str()[3 + mnc_digits..])
    }
}

digits_identity!(Imsi, "imsi", 6, 15, "IMSI");

/// An IMEI: 15 decimal digits, which are a type allocation code of eight digits, a
/// serial number of six, and a check digit or a spare digit (3GPP TS 23.003 clause 6.2.1).
///
/// A mobile sends 0 as the last digit, the spare digit; the check digit of annex B of
/// TS 23.003 is not sent.
///
/// ```
/// use oxirush_utils::identity::Imei;
///
/// let sent = Imei::with_spare_digit("49015420323751")?;
/// assert_eq!(sent.as_str(), "490154203237510");
/// assert_eq!(Imei::with_check_digit("49015420323751")?.as_str(), "490154203237518");
/// assert_eq!((sent.tac(), sent.snr()), ("49015420", "323751"));
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Imei(Decimal<15>);

impl Imei {
    /// The IMEI of its 15 digits, whatever the last one is.
    ///
    /// # Errors
    ///
    /// `digits` are not 15 decimal digits.
    pub fn new(digits: &str) -> Result<Self, Error> {
        Decimal::new("imei", digits, 15).map(Self)
    }

    /// The IMEI of the 14 digits of a type allocation code and a serial number, with the
    /// spare digit 0 after them, as a mobile sends it.
    ///
    /// # Errors
    ///
    /// `tac_snr` are not 14 decimal digits.
    pub fn with_spare_digit(tac_snr: &str) -> Result<Self, Error> {
        Self::with_last(tac_snr, |_| 0)
    }

    /// The IMEI of the 14 digits of a type allocation code and a serial number, with
    /// their check digit after them: the Luhn digit of annex B of 3GPP TS 23.003.
    ///
    /// # Errors
    ///
    /// `tac_snr` are not 14 decimal digits.
    pub fn with_check_digit(tac_snr: &str) -> Result<Self, Error> {
        Self::with_last(tac_snr, check_digit)
    }

    fn with_last(tac_snr: &str, last: impl FnOnce(&[u8]) -> u8) -> Result<Self, Error> {
        decimal("tac-snr", tac_snr, 14, 14)?;
        let mut digits = [b'0'; 15];
        digits[..14].copy_from_slice(tac_snr.as_bytes());
        digits[14] = b'0' + last(tac_snr.as_bytes());
        Ok(Self(Decimal { digits, length: 15 }))
    }

    /// The type allocation code: the first eight digits.
    pub fn tac(&self) -> &str {
        &self.as_str()[..8]
    }

    /// The serial number: the six digits after the type allocation code.
    pub fn snr(&self) -> &str {
        &self.as_str()[8..14]
    }

    /// Whether the last digit is the check digit of the 14 before it.
    pub fn has_check_digit(&self) -> bool {
        let digits = self.as_str().as_bytes();
        digits[14] - b'0' == check_digit(&digits[..14])
    }
}

/// The check digit of 14 ASCII digits: the digit that makes their Luhn sum a multiple of
/// ten, with each second digit doubled.
fn check_digit(tac_snr: &[u8]) -> u8 {
    let mut sum = 0;
    for (index, digit) in tac_snr.iter().enumerate() {
        let digit = digit - b'0';
        sum += match index % 2 {
            0 => digit,
            // Twice the digit, as the sum of its two digits.
            _ => digit * 2 / 10 + digit * 2 % 10,
        };
    }
    (10 - sum % 10) % 10
}

digits_identity!(Imei, "imei", 15, 15, "IMEI");

/// An IMEISV: 16 decimal digits, which are a type allocation code of eight digits, a
/// serial number of six and a software version number of two (3GPP TS 23.003 clause
/// 6.2.2).
///
/// ```
/// use oxirush_utils::identity::Imeisv;
///
/// let imeisv = Imeisv::new("4901542032375101")?;
/// assert_eq!((imeisv.tac(), imeisv.snr(), imeisv.svn()), ("49015420", "323751", "01"));
///
/// // In a 5GS mobile identity, where the type of identity of an IMEISV is 5
/// let octets = imeisv.to_nas_octets(5)?;
/// assert_eq!(octets, [0x45, 0x09, 0x51, 0x24, 0x30, 0x32, 0x57, 0x01, 0xf1]);
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Imeisv(Decimal<16>);

impl Imeisv {
    /// The IMEISV of its 16 digits.
    ///
    /// # Errors
    ///
    /// `digits` are not 16 decimal digits.
    pub fn new(digits: &str) -> Result<Self, Error> {
        Decimal::new("imeisv", digits, 16).map(Self)
    }

    /// The type allocation code: the first eight digits.
    pub fn tac(&self) -> &str {
        &self.as_str()[..8]
    }

    /// The serial number: the six digits after the type allocation code.
    pub fn snr(&self) -> &str {
        &self.as_str()[8..14]
    }

    /// The software version number: the last two digits.
    pub fn svn(&self) -> &str {
        &self.as_str()[14..]
    }
}

digits_identity!(Imeisv, "imeisv", 16, 16, "IMEISV");
