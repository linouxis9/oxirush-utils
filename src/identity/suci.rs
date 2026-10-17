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

//! The SUCI of an IMSI as a value, with its routing indicator and its protection scheme.
//! What conceals an identity and what reads a concealed one are no part of it.

use super::digits::decimal;
#[cfg(feature = "alloc")]
use super::nai::{MAC_TAG, PROFILE_A_KEY, PROFILE_B_KEY};
use super::plmn::FILLER;
use super::{Error, Fault};
#[cfg(feature = "alloc")]
use super::{Faults, Imsi, Plmn, Tolerated, tbcd};
#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};
use core::fmt;
use core::str::FromStr;

/// The protection scheme of a SUCI, by its identifier (3GPP TS 24.501 table 9.11.3.4.1,
/// TS 33.501 annex C.1): 0 is the null scheme, 1 and 2 the profiles A and B of ECIES,
/// and 12 to 15 are schemes of the home operator. 3 to 11 are reserved for schemes of a
/// later version, and are no scheme here.
///
/// ```
/// use oxirush_utils::identity::ProtectionScheme;
///
/// assert_eq!(ProtectionScheme::PROFILE_A.identifier(), 1);
/// assert!(ProtectionScheme::new(0)?.is_null());
/// assert!(ProtectionScheme::new(12)?.is_of_the_operator());
/// assert!(ProtectionScheme::new(3).is_err());
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtectionScheme(u8);

impl ProtectionScheme {
    /// The null scheme: the identity is not concealed.
    pub const NULL: Self = Self(0);
    /// ECIES profile A.
    pub const PROFILE_A: Self = Self(1);
    /// ECIES profile B.
    pub const PROFILE_B: Self = Self(2);

    /// The scheme of an identifier.
    ///
    /// # Errors
    ///
    /// `identifier` is 3 to 11, which are reserved, or above 15.
    pub fn new(identifier: u8) -> Result<Self, Error> {
        let member = "protection-scheme";
        match identifier {
            0..=2 | 12..=15 => Ok(Self(identifier)),
            3..=11 => {
                let expected = "0, 1, 2 or 12 to 15: 3 to 11 are reserved";
                let given = identifier.into();
                Err(Error::new(member, Fault::Value { given, expected }))
            }
            given => {
                let (given, max) = (given.into(), 15);
                Err(Error::new(member, Fault::Range { given, max }))
            }
        }
    }

    /// The identifier: 0, 1, 2 or 12 to 15.
    pub fn identifier(self) -> u8 {
        self.0
    }

    /// Whether it is the null scheme.
    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Whether it is a scheme of the home operator: 12 to 15.
    pub fn is_of_the_operator(self) -> bool {
        self.0 >= 12
    }
}

impl fmt::Debug for ProtectionScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.0 {
            0 => "null",
            1 => "profile A",
            2 => "profile B",
            _ => "of the operator",
        };
        write!(f, "ProtectionScheme({}: {name})", self.0)
    }
}

/// The routing indicator of a SUCI: one to four decimal digits, each of which counts
/// (3GPP TS 23.003 clause 2.2B): `012` is not `12`, and `0` is the one that says no
/// indicator is configured.
///
/// ```
/// use oxirush_utils::identity::RoutingIndicator;
///
/// let indicator = RoutingIndicator::new("678")?;
/// assert_eq!(indicator.to_nas_octets(), [0x76, 0xf8]);
/// assert_eq!(RoutingIndicator::default().to_nas_octets(), [0xf0, 0xff]);
/// assert_ne!(RoutingIndicator::new("012")?, RoutingIndicator::new("12")?);
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutingIndicator {
    /// The digits as ASCII, then `0`.
    digits: [u8; 4],
    length: u8,
}

impl RoutingIndicator {
    /// The routing indicator of its digits.
    ///
    /// # Errors
    ///
    /// `digits` are not one to four decimal digits.
    pub fn new(digits: &str) -> Result<Self, Error> {
        decimal("routing-indicator", digits, 1, 4)?;
        let mut all = [b'0'; 4];
        all[..digits.len()].copy_from_slice(digits.as_bytes());
        Ok(Self {
            digits: all,
            length: digits.len() as u8,
        })
    }

    /// The digits.
    pub fn as_str(&self) -> &str {
        // The digits are ASCII.
        core::str::from_utf8(&self.digits[..usize::from(self.length)]).unwrap_or_default()
    }

    /// The two octets of the routing indicator in a 5GS mobile identity: its digits in
    /// BCD, the first in the low half of the first octet, and `1111` for each digit that
    /// it does not have (3GPP TS 24.501 table 9.11.3.4.1).
    pub fn to_nas_octets(&self) -> [u8; 2] {
        let half = |index: usize| match index < usize::from(self.length) {
            true => self.digits[index] - b'0',
            false => FILLER,
        };
        [half(1) << 4 | half(0), half(3) << 4 | half(2)]
    }

    /// The routing indicator that two octets hold.
    ///
    /// # Errors
    ///
    /// The first half octet is no digit, a half octet after a digit is neither a digit
    /// nor `1111`, or a digit follows a `1111`.
    pub fn from_nas_octets(octets: [u8; 2]) -> Result<Self, Error> {
        let member = "routing-indicator";
        let halves = [
            octets[0] & 0x0f,
            octets[0] >> 4,
            octets[1] & 0x0f,
            octets[1] >> 4,
        ];
        let length = halves.iter().take_while(|half| **half <= 9).count();
        if length == 0 {
            return Err(Error::new(member, Fault::Digit { given: halves[0] }));
        }
        if let Some(given) = halves[length..].iter().find(|half| **half != FILLER) {
            return Err(Error::new(member, Fault::EndMark { given: *given }));
        }
        let mut digits = [b'0'; 4];
        for (digit, half) in digits.iter_mut().zip(&halves[..length]) {
            *digit = b'0' + half;
        }
        Ok(Self {
            digits,
            length: length as u8,
        })
    }
}

impl Default for RoutingIndicator {
    /// `0`: what a mobile sends when none is configured.
    fn default() -> Self {
        Self {
            digits: [b'0'; 4],
            length: 1,
        }
    }
}

impl fmt::Display for RoutingIndicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for RoutingIndicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RoutingIndicator({self})")
    }
}

impl FromStr for RoutingIndicator {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        Self::new(text)
    }
}

/// The SUCI of an IMSI (3GPP TS 23.003 clause 2.2B): the PLMN of the home network, the
/// routing indicator, the protection scheme, the identifier of the public key of the
/// home network, and the scheme output, which is the MSIN under the null scheme and
/// octets that only the home network reads otherwise.
///
/// It is the value of the parts and of their two forms: the 5GS mobile identity of
/// TS 24.501 figure 9.11.3.4.3, and the text of its parts with a hyphen between them,
/// after `suci`, as the service-based interfaces write a SUCI (`SupiOrSuci` in the
/// OpenAPI of TS 29.571). Concealing an IMSI and reading a concealed one need keys, and
/// are no part of this crate. A SUCI in the form of a NAI, which a SUPI that is no IMSI
/// has, is not this type.
///
/// ```
/// use oxirush_utils::identity::{ImsiSuci, Plmn, RoutingIndicator};
///
/// // The example of TS 23.003 clause 2.2B: IMSI 234150999999999, routing indicator 678
/// let plmn = Plmn::new("234", "15")?;
/// let suci = ImsiSuci::null(plmn, RoutingIndicator::new("678")?, "0999999999")?;
/// assert_eq!(suci.to_string(), "suci-0-234-15-678-0-0-0999999999");
/// assert_eq!(suci.imsi().unwrap().as_str(), "234150999999999");
///
/// let octets = suci.to_nas_octets();
/// assert_eq!(octets, [0x01, 0x32, 0xf4, 0x51, 0x76, 0xf8, 0x00, 0x00, 0x90, 0x99, 0x99, 0x99, 0x99]);
/// assert_eq!(ImsiSuci::from_nas_octets(&octets)?, suci);
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImsiSuci {
    plmn: Plmn,
    routing_indicator: RoutingIndicator,
    scheme: ProtectionScheme,
    key_id: u8,
    /// The MSIN as ASCII digits under the null scheme, and the scheme output otherwise.
    output: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl ImsiSuci {
    /// The SUCI of an IMSI that is not concealed: the null scheme, the key identifier
    /// 0, and the MSIN as the scheme output.
    ///
    /// # Errors
    ///
    /// `msin` is not decimal digits, has none, or makes an IMSI of more than 15 digits
    /// with the PLMN.
    pub fn null(
        plmn: Plmn,
        routing_indicator: RoutingIndicator,
        msin: &str,
    ) -> Result<Self, Error> {
        decimal("msin", msin, 1, 15 - plmn.digits().len())?;
        Ok(Self {
            plmn,
            routing_indicator,
            scheme: ProtectionScheme::NULL,
            key_id: 0,
            output: msin.as_bytes().to_vec(),
        })
    }

    /// The SUCI of an IMSI that a scheme conceals: the scheme, the identifier of the
    /// public key of the home network that it used, and its output.
    ///
    /// # Errors
    ///
    /// `scheme` is the null scheme, `key_id` is 0, which a SUCI has "if and only if"
    /// its scheme is the null one (TS 23.003 clause 2.2B), or 255, which is reserved
    /// (TS 24.501 table 9.11.3.4.1), or `output` has another length than its scheme
    /// gives it: for profile A, 32 octets of public key, the ciphertext of the MSIN in
    /// packed BCD and 8 of MAC tag; for profile B, 33 of public key; for a scheme of the
    /// operator, 3000 octets at most beside the MSIN (TS 33.501 annex C.1). Open5GS takes
    /// the key identifiers 1 to 254, and the UDM of free5GC refuses 0.
    pub fn concealed(
        plmn: Plmn,
        routing_indicator: RoutingIndicator,
        scheme: ProtectionScheme,
        key_id: u8,
        output: Vec<u8>,
    ) -> Result<Self, Error> {
        let suci = Self::of_any_output(plmn, routing_indicator, scheme, key_id, output)?;
        match suci.output_length() {
            Some(fault) => Err(fault),
            None => Ok(suci),
        }
    }

    /// The SUCI of an IMSI that a scheme conceals, whatever the length of its output:
    /// [`concealed`](Self::concealed) without the check of that length.
    fn of_any_output(
        plmn: Plmn,
        routing_indicator: RoutingIndicator,
        scheme: ProtectionScheme,
        key_id: u8,
        output: Vec<u8>,
    ) -> Result<Self, Error> {
        if scheme.is_null() {
            let expected = "a scheme that conceals: 1, 2 or 12 to 15";
            return Err(Error::new(
                "protection-scheme",
                Fault::Value { given: 0, expected },
            ));
        }
        if matches!(key_id, 0 | 255) {
            let expected = "1 to 254 for a scheme that conceals";
            let given = key_id.into();
            let fault = Fault::Value { given, expected };
            return Err(Error::new("home-network-public-key-identifier", fault));
        }
        Ok(Self {
            plmn,
            routing_indicator,
            scheme,
            key_id,
            output,
        })
    }

    /// Why the output of a scheme that conceals has another length than its scheme gives
    /// it; `None` for one of its length. The output of a profile of ECIES is the
    /// ephemeral public key, the ciphertext of the scheme input and the MAC tag, and that
    /// of a scheme of the operator 3000 octets at most beside its input (TS 33.501 annex
    /// C.1). The input is the MSIN in packed BCD (annex C.3.1): one octet for two of its
    /// digits.
    fn output_length(&self) -> Option<Error> {
        let input = (15 - self.plmn.digits().len()).div_ceil(2);
        let (shortest, longest) = match self.scheme {
            ProtectionScheme::PROFILE_A => {
                (PROFILE_A_KEY + 1 + MAC_TAG, PROFILE_A_KEY + input + MAC_TAG)
            }
            ProtectionScheme::PROFILE_B => {
                (PROFILE_B_KEY + 1 + MAC_TAG, PROFILE_B_KEY + input + MAC_TAG)
            }
            _ => (1, 3000 + input),
        };
        let length = self.output.len();
        let unit = "octets";
        (!(shortest..=longest).contains(&length))
            .then(|| Error::length("scheme-output", unit, length, shortest, longest))
    }

    /// The SUCI of an IMSI that is not concealed, for an MNC of `mnc_digits` digits and
    /// the routing indicator 0.
    ///
    /// # Errors
    ///
    /// `mnc_digits` is neither 2 nor 3, or the IMSI has no digit after its PLMN.
    pub fn of(imsi: Imsi, mnc_digits: usize) -> Result<Self, Error> {
        Self::null(
            imsi.plmn(mnc_digits)?,
            RoutingIndicator::default(),
            imsi.msin(mnc_digits)?,
        )
    }

    /// The PLMN of the home network.
    pub fn plmn(&self) -> Plmn {
        self.plmn
    }

    /// The routing indicator.
    pub fn routing_indicator(&self) -> RoutingIndicator {
        self.routing_indicator
    }

    /// The protection scheme.
    pub fn protection_scheme(&self) -> ProtectionScheme {
        self.scheme
    }

    /// The identifier of the public key of the home network: 0 under the null scheme.
    pub fn home_network_public_key_identifier(&self) -> u8 {
        self.key_id
    }

    /// The MSIN, under the null scheme.
    pub fn msin(&self) -> Option<&str> {
        match self.scheme.is_null() {
            true => core::str::from_utf8(&self.output).ok(),
            false => None,
        }
    }

    /// The output of a scheme that conceals: the octets that the home network reads.
    pub fn concealed_output(&self) -> Option<&[u8]> {
        (!self.scheme.is_null()).then_some(&self.output[..])
    }

    /// The IMSI, under the null scheme.
    pub fn imsi(&self) -> Option<Imsi> {
        let mut digits = String::from(self.plmn.digits());
        digits.push_str(self.msin()?);
        Imsi::new(&digits).ok()
    }

    /// The octets of the SUCI as a 5GS mobile identity: `01`, the PLMN in the order of
    /// NAS, the routing indicator, the protection scheme, the key identifier, and the
    /// scheme output, which is the MSIN as a TBCD string under the null scheme (3GPP
    /// TS 24.501 clause 9.11.3.4, figures 9.11.3.4.3 and 9.11.3.4.3a).
    pub fn to_nas_octets(&self) -> Vec<u8> {
        let mut octets = Vec::with_capacity(8 + self.output.len());
        octets.push(0x01);
        octets.extend_from_slice(&self.plmn.to_nas_octets());
        octets.extend_from_slice(&self.routing_indicator.to_nas_octets());
        octets.extend_from_slice(&[self.scheme.0, self.key_id]);
        match self.msin() {
            // The digits of an MSIN are those of a TBCD string.
            Some(msin) => octets.extend(tbcd::encode(msin).unwrap_or_default()),
            None => octets.extend_from_slice(&self.output),
        }
        octets
    }

    /// The SUCI that `octets` hold as a 5GS mobile identity.
    ///
    /// # Errors
    ///
    /// There are fewer than nine octets, the first is not `01`, the PLMN, the routing
    /// indicator or the MSIN are not their digits, bits 8 to 5 of the octet of the
    /// scheme are not zero, the scheme is a reserved one, or the key identifier is not
    /// one of the scheme.
    pub fn from_nas_octets(octets: &[u8]) -> Result<Self, Error> {
        let (suci, faults) = Self::from_nas_octets_tolerant(octets)?;
        match faults.first() {
            Some(fault) => Err(*fault),
            None => Ok(suci),
        }
    }

    /// [`from_nas_octets`](Self::from_nas_octets), for octets as other implementations
    /// send them: the value, and what a sender by the clause would have written
    /// otherwise.
    ///
    /// Four departures are read, each a fault: a spare bit of the first octet that is
    /// 1, a spare bit of the octet of the scheme that is 1 (3GPP TS 24.007 clause
    /// 11.1.4: a receiver accepts a spare bit as 0 or as 1; Open5GS reads neither bit 4
    /// of the first octet nor the spare half of the scheme), a SUPI format of 4 to 7,
    /// which TS 24.501 table 9.11.3.4.1 has a receiver interpret as IMSI, as free5GC
    /// does, and a scheme output of a scheme that conceals of another length than its
    /// scheme gives it. That output is opaque to every node but the home network,
    /// whose de-concealment decides (TS 33.501 annex C.3.3; TS 33.514 clause 4.2.1.3
    /// has it refuse an uncompressed ephemeral key of profile B): the value holds it
    /// as it was received, and writes it back, so that a node passes it on.
    ///
    /// # Errors
    ///
    /// Octets that are no SUCI of an IMSI however they are read: another type of
    /// identity, the SUPI format of a NAI, and what the parts refuse.
    pub fn from_nas_octets_tolerant(octets: &[u8]) -> Result<Tolerated<Self>, Error> {
        let member = "suci";
        let [first, p1, p2, p3, r1, r2, scheme, key_id, output @ ..] = octets else {
            return Err(Error::length(member, "octets", octets.len(), 9, usize::MAX));
        };
        if output.is_empty() {
            return Err(Error::length(member, "octets", octets.len(), 9, usize::MAX));
        }
        let mut faults = Faults::new();
        // The type of identity is that of a SUCI, and the SUPI format that of an IMSI.
        let (format, kind) = ((first >> 4) & 0x07, first & 0x07);
        if kind != 1 || matches!(format, 1..=3) {
            let fault = Fault::Octet {
                given: *first,
                expected: 0x01,
            };
            return Err(Error::new(member, fault));
        }
        if format != 0 {
            let expected = "0, the SUPI format of an IMSI";
            let given = format.into();
            faults.push(Error::new("supi-format", Fault::Value { given, expected }));
        }
        if first & 0x88 != 0 {
            faults.push(Error::new(member, Fault::Spare { given: *first }));
        }
        if scheme & 0xf0 != 0 {
            faults.push(Error::new(
                "protection-scheme",
                Fault::Spare { given: *scheme },
            ));
        }
        let plmn = Plmn::from_nas_octets([*p1, *p2, *p3])?;
        let routing_indicator = RoutingIndicator::from_nas_octets([*r1, *r2])?;
        let scheme = ProtectionScheme::new(scheme & 0x0f)?;
        let suci = match scheme.is_null() {
            true => {
                if *key_id != 0 {
                    let expected = "0 for the null scheme";
                    let given = (*key_id).into();
                    let fault = Fault::Value { given, expected };
                    return Err(Error::new("home-network-public-key-identifier", fault));
                }
                let msin = tbcd::decode(output).map_err(|error| error.of("msin"))?;
                Self::null(plmn, routing_indicator, &msin)?
            }
            false => {
                let output = output.to_vec();
                let suci = Self::of_any_output(plmn, routing_indicator, scheme, *key_id, output)?;
                // The output is opaque to every node but the home network, whose
                // de-concealment decides: one of another length is read, as a fault.
                if let Some(fault) = suci.output_length() {
                    faults.push(fault);
                }
                suci
            }
        };
        Ok((suci, faults))
    }
}

#[cfg(feature = "alloc")]
impl fmt::Display for ImsiSuci {
    /// `suci-0-`, then the MCC, the MNC, the routing indicator, the scheme, the key
    /// identifier and the scheme output, with a hyphen between them, by the pattern of
    /// `SupiOrSuci` in the OpenAPI of 3GPP TS 29.571: the scheme is one hexadecimal
    /// digit, so that the first scheme of the operator is `c`, the key identifier is in
    /// decimal, and the scheme output is the MSIN, or octets in hexadecimal.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (plmn, routing, scheme, key) = (
            self.plmn,
            self.routing_indicator,
            self.scheme.0,
            self.key_id,
        );
        write!(f, "suci-0-{plmn}-{routing}-{scheme:x}-{key}-")?;
        match self.msin() {
            Some(msin) => f.write_str(msin),
            None => self
                .output
                .iter()
                .try_for_each(|octet| write!(f, "{octet:02x}")),
        }
    }
}

#[cfg(feature = "alloc")]
impl fmt::Debug for ImsiSuci {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ImsiSuci({self})")
    }
}

#[cfg(feature = "alloc")]
impl FromStr for ImsiSuci {
    type Err = Error;

    /// The SUCI that is written as [`Display`](fmt::Display) writes one. The
    /// hexadecimal digits are read in either case.
    fn from_str(text: &str) -> Result<Self, Error> {
        let expected = "suci-0-, the MCC, the MNC, the routing indicator, the scheme as one \
                        hexadecimal digit, the key identifier in decimal and the scheme output, \
                        as suci-0-208-93-0-0-0-0000000001";
        let form = Error::new("suci", Fault::Form { expected });
        let rest = text.strip_prefix("suci-0-").ok_or(form)?;
        let mut parts = rest.splitn(6, '-');
        let mut part = || parts.next().ok_or(form);
        let plmn = Plmn::new(part()?, part()?)?;
        let routing_indicator = RoutingIndicator::new(part()?)?;
        let scheme = match part()?.as_bytes() {
            [digit] => (*digit as char).to_digit(16).ok_or(form)?,
            _ => return Err(form),
        };
        let scheme = ProtectionScheme::new(scheme as u8)?;
        // A number in decimal is written in one way: without a zero before it.
        let key_id = part()?;
        let decimal = key_id.bytes().all(|digit| digit.is_ascii_digit())
            && (key_id == "0" || !key_id.starts_with('0'));
        let key_id = match decimal {
            true => key_id.parse::<u8>().map_err(|_| form)?,
            false => return Err(form),
        };
        let output = part()?;
        if scheme.is_null() {
            if key_id != 0 {
                let expected = "0 for the null scheme";
                let given = key_id.into();
                let fault = Fault::Value { given, expected };
                return Err(Error::new("home-network-public-key-identifier", fault));
            }
            return Self::null(plmn, routing_indicator, output);
        }
        let hexadecimal =
            output.len() % 2 == 0 && output.bytes().all(|digit| digit.is_ascii_hexdigit());
        if !hexadecimal {
            return Err(form);
        }
        let octets = (0..output.len()).step_by(2);
        let octets = octets.map(|at| u8::from_str_radix(&output[at..at + 2], 16).map_err(|_| form));
        Self::concealed(
            plmn,
            routing_indicator,
            scheme,
            key_id,
            octets.collect::<Result<_, _>>()?,
        )
    }
}
