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

//! The PLMN identity, and its two codings.

use super::digits::decimal;
use super::{Error, Fault};
use core::fmt;
use core::str::FromStr;

/// The filler of a coding: the half octet `1111`.
pub(crate) const FILLER: u8 = 0x0f;

/// A PLMN identity: a mobile country code of three decimal digits and a mobile network
/// code of two or three (3GPP TS 23.003 clause 2.2).
///
/// The number of digits of the MNC is part of the value: `208-93` and `208-093` are two
/// PLMNs. A PLMN is written and read as `208-93`.
///
/// Three octets hold a PLMN in two orders, which differ for an MNC of three digits:
///
/// | Coding | Clause | 208-93 | 310-410 |
/// |--------|--------|--------|---------|
/// | [`to_nas_octets`](Self::to_nas_octets) | TS 24.008 clause 10.5.1.3, which TS 24.501 and TS 24.301 use | `02 f8 39` | `13 00 14` |
/// | [`to_ap_octets`](Self::to_ap_octets) | TS 38.413 clause 9.3.3.5 and TS 36.413 clause 9.2.3.8 | `02 f8 39` | `13 40 01` |
///
/// ```
/// use oxirush_utils::identity::Plmn;
///
/// let plmn: Plmn = "310-410".parse()?;
/// assert_eq!((plmn.mcc(), plmn.mnc()), ("310", "410"));
///
/// // In a NAS message, and in an NGAP or S1AP message
/// assert_eq!(plmn.to_nas_octets(), [0x13, 0x00, 0x14]);
/// assert_eq!(plmn.to_ap_octets(), [0x13, 0x40, 0x01]);
/// assert_eq!(Plmn::from_ap_octets([0x13, 0x40, 0x01])?, plmn);
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Plmn {
    /// The digits of the MCC, then those of the MNC, as ASCII: the sixth is `0` for an
    /// MNC of two digits.
    digits: [u8; 6],
    /// How many digits the MNC has: 2 or 3.
    mnc_length: u8,
}

impl Plmn {
    /// The PLMN of an MCC and an MNC.
    ///
    /// # Errors
    ///
    /// `mcc` is not three decimal digits, or `mnc` not two or three.
    pub fn new(mcc: &str, mnc: &str) -> Result<Self, Error> {
        decimal("mcc", mcc, 3, 3)?;
        decimal("mnc", mnc, 2, 3)?;
        let mut all = [b'0'; 6];
        all[..3].copy_from_slice(mcc.as_bytes());
        all[3..3 + mnc.len()].copy_from_slice(mnc.as_bytes());
        Ok(Self {
            digits: all,
            mnc_length: mnc.len() as u8,
        })
    }

    /// The PLMN of the digits of an MCC and of an MNC, each a number from 0 to 9.
    ///
    /// # Errors
    ///
    /// `mnc` has not two or three digits, or a digit is above 9.
    pub fn from_digits(mcc: [u8; 3], mnc: &[u8]) -> Result<Self, Error> {
        if !(2..=3).contains(&mnc.len()) {
            return Err(Error::length("mnc", "digits", mnc.len(), 2, 3));
        }
        let mut all = [b'0'; 6];
        let members = ["mcc", "mcc", "mcc", "mnc", "mnc", "mnc"];
        for (index, digit) in mcc.iter().chain(mnc).enumerate() {
            if *digit > 9 {
                let fault = Fault::Digit { given: *digit };
                return Err(Error::new(members[index], fault));
            }
            all[index] = b'0' + digit;
        }
        Ok(Self {
            digits: all,
            mnc_length: mnc.len() as u8,
        })
    }

    /// The mobile country code: three decimal digits.
    pub fn mcc(&self) -> &str {
        // The digits are ASCII.
        core::str::from_utf8(&self.digits[..3]).unwrap_or_default()
    }

    /// The mobile network code: two or three decimal digits.
    pub fn mnc(&self) -> &str {
        let digits = &self.digits[3..3 + usize::from(self.mnc_length)];
        core::str::from_utf8(digits).unwrap_or_default()
    }

    /// The digits of the MCC and of the MNC as one string: `20893`, `310410`.
    pub fn digits(&self) -> &str {
        let digits = &self.digits[..3 + usize::from(self.mnc_length)];
        core::str::from_utf8(digits).unwrap_or_default()
    }

    /// The digit at `index` of the six, as a number.
    fn digit(&self, index: usize) -> u8 {
        self.digits[index] - b'0'
    }

    /// The octets of the PLMN in a NAS message: the order of 3GPP TS 24.008 clause
    /// 10.5.1.3, in which the second octet has the third digit of the MNC, or the filler
    /// `1111` for an MNC of two digits. `310-410` is `13 00 14`.
    pub fn to_nas_octets(self) -> [u8; 3] {
        let mnc_3 = match self.mnc_length {
            3 => self.digit(5),
            _ => FILLER,
        };
        [
            self.digit(1) << 4 | self.digit(0),
            mnc_3 << 4 | self.digit(2),
            self.digit(4) << 4 | self.digit(3),
        ]
    }

    /// The PLMN that `octets` hold in the order of [`to_nas_octets`](Self::to_nas_octets).
    ///
    /// # Errors
    ///
    /// A half octet is no decimal digit, where it is not the filler in the place of the
    /// third digit of the MNC. TS 24.008 clause 10.5.1.3 has a receiver treat a location
    /// area with such an MCC or MNC as deleted: it is no PLMN.
    pub fn from_nas_octets(octets: [u8; 3]) -> Result<Self, Error> {
        let [first, second, third] = octets;
        let mcc = [first & 0x0f, first >> 4, second & 0x0f];
        let mnc = [third & 0x0f, third >> 4, second >> 4];
        match mnc[2] {
            FILLER => Self::from_digits(mcc, &mnc[..2]),
            _ => Self::from_digits(mcc, &mnc),
        }
    }

    /// The octets of the PLMN in an NGAP or an S1AP message: the order of 3GPP TS 38.413
    /// clause 9.3.3.5 and of TS 36.413 clause 9.2.3.8, "3 digits from MCC followed by
    /// either a filler digit plus 2 digits from MNC" or "3 digits from MNC", with digit
    /// 2n-1 in bits 4 to 1 of octet n and digit 2n in its bits 8 to 5. The second octet
    /// has the first digit of an MNC of three digits: `310-410` is `13 40 01`.
    pub fn to_ap_octets(self) -> [u8; 3] {
        let [d4, d5, d6] = match self.mnc_length {
            3 => [self.digit(3), self.digit(4), self.digit(5)],
            _ => [FILLER, self.digit(3), self.digit(4)],
        };
        [
            self.digit(1) << 4 | self.digit(0),
            d4 << 4 | self.digit(2),
            d6 << 4 | d5,
        ]
    }

    /// The PLMN that `octets` hold in the order of [`to_ap_octets`](Self::to_ap_octets).
    ///
    /// # Errors
    ///
    /// A half octet is no decimal digit, where it is not the filler before an MNC of
    /// two digits.
    pub fn from_ap_octets(octets: [u8; 3]) -> Result<Self, Error> {
        let [first, second, third] = octets;
        let mcc = [first & 0x0f, first >> 4, second & 0x0f];
        let mnc = [second >> 4, third & 0x0f, third >> 4];
        match mnc[0] {
            FILLER => Self::from_digits(mcc, &mnc[1..]),
            _ => Self::from_digits(mcc, &mnc),
        }
    }
}

impl fmt::Display for Plmn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.mcc(), self.mnc())
    }
}

impl fmt::Debug for Plmn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Plmn({self})")
    }
}

impl FromStr for Plmn {
    type Err = Error;

    /// The PLMN that is written as `208-93`.
    fn from_str(text: &str) -> Result<Self, Error> {
        let expected = "an MCC of three digits, a hyphen and an MNC of two or three, as 208-93";
        let (mcc, mnc) = text
            .split_once('-')
            .ok_or(Error::new("plmn", Fault::Form { expected }))?;
        Self::new(mcc, mnc)
    }
}
