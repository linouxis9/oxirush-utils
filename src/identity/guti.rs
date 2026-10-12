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

//! The temporary identities of both systems: the 5G-GUTI and the 5G-S-TMSI, the GUTI of
//! EPS and its S-TMSI.

use super::{Error, Fault, Plmn, Tolerated};
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

/// Check that `value` is `max` at most.
fn within(member: &'static str, value: u64, max: u64) -> Result<(), Error> {
    match value <= max {
        true => Ok(()),
        false => Err(Error::new(member, Fault::Range { given: value, max })),
    }
}

/// The `N` octets of an identity, when `octets` are that many.
fn sized<const N: usize>(member: &'static str, octets: &[u8]) -> Result<[u8; N], Error> {
    <[u8; N]>::try_from(octets).map_err(|_| Error::length(member, "octets", octets.len(), N, N))
}

/// Check the first octet of a mobile identity of 5GS that has no digits: `1111`, the
/// spare bit 4, and the type of identity. A spare bit that is 1 is a fault of a
/// tolerant reading, and other bits are no such identity.
fn first_octet(
    member: &'static str,
    given: u8,
    expected: u8,
    faults: &mut Vec<Error>,
) -> Result<(), Error> {
    if given == expected {
        Ok(())
    } else if given & 0xf7 == expected {
        faults.push(Error::new(member, Fault::Spare { given }));
        Ok(())
    } else {
        Err(Error::new(member, Fault::Octet { given, expected }))
    }
}

/// The value of a tolerant reading when it has no fault, and its first fault otherwise.
fn strict<T>(read: Tolerated<T>) -> Result<T, Error> {
    match read.1.first() {
        Some(fault) => Err(*fault),
        None => Ok(read.0),
    }
}

/// A 5G-S-TMSI: the AMF Set ID of 10 bits, the AMF Pointer of 6 and the 5G-TMSI of 32
/// (3GPP TS 23.003 clause 2.10.1).
///
/// ```
/// use oxirush_utils::identity::STmsi5g;
///
/// let s_tmsi = STmsi5g::new(0x3fe, 0x01, 0xdead_beef)?;
/// assert_eq!(s_tmsi.to_nas_octets(), [0xf4, 0xff, 0x81, 0xde, 0xad, 0xbe, 0xef]);
/// assert_eq!(s_tmsi.to_string(), "5g-s-tmsi-ff81deadbeef");
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct STmsi5g {
    amf_set_id: u16,
    amf_pointer: u8,
    tmsi: u32,
}

impl STmsi5g {
    /// The 5G-S-TMSI of its parts.
    ///
    /// # Errors
    ///
    /// `amf_set_id` is above 1023, or `amf_pointer` above 63.
    pub fn new(amf_set_id: u16, amf_pointer: u8, tmsi: u32) -> Result<Self, Error> {
        within("amf-set-id", amf_set_id.into(), 0x3ff)?;
        within("amf-pointer", amf_pointer.into(), 0x3f)?;
        Ok(Self {
            amf_set_id,
            amf_pointer,
            tmsi,
        })
    }

    /// The AMF Set ID: 10 bits.
    pub fn amf_set_id(&self) -> u16 {
        self.amf_set_id
    }

    /// The AMF Pointer: 6 bits.
    pub fn amf_pointer(&self) -> u8 {
        self.amf_pointer
    }

    /// The 5G-TMSI.
    pub fn tmsi(&self) -> u32 {
        self.tmsi
    }

    /// The 48 bits of the 5G-S-TMSI as a number: the AMF Set ID, the AMF Pointer, then
    /// the 5G-TMSI, as NGAP and RRC carry it.
    pub fn to_u64(&self) -> u64 {
        u64::from(self.amf_set_id) << 38 | u64::from(self.amf_pointer) << 32 | u64::from(self.tmsi)
    }

    /// The 5G-S-TMSI of its 48 bits as a number.
    ///
    /// # Errors
    ///
    /// `bits` has more than 48 bits.
    pub fn from_u64(bits: u64) -> Result<Self, Error> {
        within("5g-s-tmsi", bits, (1 << 48) - 1)?;
        Ok(Self {
            amf_set_id: (bits >> 38) as u16,
            amf_pointer: (bits >> 32) as u8 & 0x3f,
            tmsi: bits as u32,
        })
    }

    /// The six octets after the first of the mobile identity.
    fn parts(&self) -> [u8; 6] {
        let [_, _, parts @ ..] = self.to_u64().to_be_bytes();
        parts
    }

    fn from_parts(parts: [u8; 6]) -> Self {
        let [a, b, c, d, e, f] = parts;
        let bits = u64::from_be_bytes([0, 0, a, b, c, d, e, f]);
        Self {
            amf_set_id: (bits >> 38) as u16,
            amf_pointer: (bits >> 32) as u8 & 0x3f,
            tmsi: bits as u32,
        }
    }

    /// The seven octets of the 5G-S-TMSI as a 5GS mobile identity: `f4`, then the 48
    /// bits (3GPP TS 24.501 clause 9.11.3.4, figure 9.11.3.4.5).
    pub fn to_nas_octets(&self) -> [u8; 7] {
        let [a, b, c, d, e, f] = self.parts();
        [0xf4, a, b, c, d, e, f]
    }

    /// The 5G-S-TMSI that `octets` hold as a 5GS mobile identity.
    ///
    /// # Errors
    ///
    /// `octets` are not seven, or the first is not `f4`.
    pub fn from_nas_octets(octets: &[u8]) -> Result<Self, Error> {
        strict(Self::from_nas_octets_tolerant(octets)?)
    }

    /// [`from_nas_octets`](Self::from_nas_octets), for octets as other implementations
    /// send them: the value, and what a sender by the clause would have written
    /// otherwise.
    ///
    /// One departure is read: the spare bit 4 of the first octet may be 1, and is then
    /// a fault. 3GPP TS 24.007 clause 11.1.4 has a receiver accept a spare bit as 0 or
    /// as 1.
    ///
    /// # Errors
    ///
    /// `octets` are not seven, or the first is not that of a 5G-S-TMSI but for its
    /// spare bit.
    pub fn from_nas_octets_tolerant(octets: &[u8]) -> Result<Tolerated<Self>, Error> {
        let [first, parts @ ..] = sized::<7>("5g-s-tmsi", octets)?;
        let mut faults = Vec::new();
        first_octet("5g-s-tmsi", first, 0xf4, &mut faults)?;
        Ok((Self::from_parts(parts), faults))
    }
}

impl fmt::Display for STmsi5g {
    /// `5g-s-tmsi-` and the 48 bits in hexadecimal.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "5g-s-tmsi-{:012x}", self.to_u64())
    }
}

impl fmt::Debug for STmsi5g {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "STmsi5g({self})")
    }
}

/// A 5G-GUTI: a PLMN, the AMF Region ID of 8 bits, and a 5G-S-TMSI (3GPP TS 23.003
/// clause 2.10.1).
///
/// It is written and read as `5g-guti-`, the digits of the MCC and of the MNC, then
/// the AMF identifier and the 5G-TMSI as 14 hexadecimal digits.
///
/// ```
/// use oxirush_utils::identity::{Guti5g, Plmn};
///
/// let guti = Guti5g::new(Plmn::new("208", "93")?, 0xca, 0x3fe, 0x01, 0xdead_beef)?;
/// let octets = guti.to_nas_octets();
/// assert_eq!(octets, [0xf2, 0x02, 0xf8, 0x39, 0xca, 0xff, 0x81, 0xde, 0xad, 0xbe, 0xef]);
/// assert_eq!(Guti5g::from_nas_octets(&octets)?, guti);
/// assert_eq!(guti.to_string(), "5g-guti-20893caff81deadbeef");
/// assert_eq!("5g-guti-20893caff81deadbeef".parse::<Guti5g>()?, guti);
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guti5g {
    plmn: Plmn,
    amf_region_id: u8,
    s_tmsi: STmsi5g,
}

impl Guti5g {
    /// The 5G-GUTI of its parts.
    ///
    /// # Errors
    ///
    /// `amf_set_id` is above 1023, or `amf_pointer` above 63.
    pub fn new(
        plmn: Plmn,
        amf_region_id: u8,
        amf_set_id: u16,
        amf_pointer: u8,
        tmsi: u32,
    ) -> Result<Self, Error> {
        let s_tmsi = STmsi5g::new(amf_set_id, amf_pointer, tmsi)?;
        Ok(Self::of(plmn, amf_region_id, s_tmsi))
    }

    /// The 5G-GUTI of a PLMN, an AMF Region ID and a 5G-S-TMSI.
    pub fn of(plmn: Plmn, amf_region_id: u8, s_tmsi: STmsi5g) -> Self {
        Self {
            plmn,
            amf_region_id,
            s_tmsi,
        }
    }

    /// The PLMN.
    pub fn plmn(&self) -> Plmn {
        self.plmn
    }

    /// The AMF Region ID.
    pub fn amf_region_id(&self) -> u8 {
        self.amf_region_id
    }

    /// The AMF Set ID: 10 bits.
    pub fn amf_set_id(&self) -> u16 {
        self.s_tmsi.amf_set_id
    }

    /// The AMF Pointer: 6 bits.
    pub fn amf_pointer(&self) -> u8 {
        self.s_tmsi.amf_pointer
    }

    /// The 5G-TMSI.
    pub fn tmsi(&self) -> u32 {
        self.s_tmsi.tmsi
    }

    /// The 5G-S-TMSI: the 5G-GUTI without its PLMN and its AMF Region ID.
    pub fn s_tmsi(&self) -> STmsi5g {
        self.s_tmsi
    }

    /// The 11 octets of the 5G-GUTI as a 5GS mobile identity: `f2`, the PLMN in the
    /// order of NAS, the AMF Region ID, then the 5G-S-TMSI (3GPP TS 24.501 clause
    /// 9.11.3.4, figure 9.11.3.4.1).
    pub fn to_nas_octets(&self) -> [u8; 11] {
        let [p1, p2, p3] = self.plmn.to_nas_octets();
        let [a, b, c, d, e, f] = self.s_tmsi.parts();
        [0xf2, p1, p2, p3, self.amf_region_id, a, b, c, d, e, f]
    }

    /// The 5G-GUTI that `octets` hold as a 5GS mobile identity.
    ///
    /// # Errors
    ///
    /// `octets` are not 11, the first is not `f2`, or the three after it are no PLMN.
    pub fn from_nas_octets(octets: &[u8]) -> Result<Self, Error> {
        strict(Self::from_nas_octets_tolerant(octets)?)
    }

    /// [`from_nas_octets`](Self::from_nas_octets), for octets as other implementations
    /// send them: the value, and what a sender by the clause would have written
    /// otherwise.
    ///
    /// One departure is read: the spare bit 4 of the first octet may be 1, and is then
    /// a fault. 3GPP TS 24.007 clause 11.1.4 has a receiver accept a spare bit as 0 or
    /// as 1.
    ///
    /// # Errors
    ///
    /// `octets` are not 11, the first is not that of a 5G-GUTI but for its spare bit,
    /// or the three after it are no PLMN.
    pub fn from_nas_octets_tolerant(octets: &[u8]) -> Result<Tolerated<Self>, Error> {
        let [first, p1, p2, p3, amf_region_id, parts @ ..] = sized::<11>("5g-guti", octets)?;
        let mut faults = Vec::new();
        first_octet("5g-guti", first, 0xf2, &mut faults)?;
        let plmn = Plmn::from_nas_octets([p1, p2, p3])?;
        let guti = Self::of(plmn, amf_region_id, STmsi5g::from_parts(parts));
        Ok((guti, faults))
    }
}

impl fmt::Display for Guti5g {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rest = u64::from(self.amf_region_id) << 48 | self.s_tmsi.to_u64();
        write!(f, "5g-guti-{}{rest:014x}", self.plmn.digits())
    }
}

impl fmt::Debug for Guti5g {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guti5g({self})")
    }
}

/// The PLMN and the 56 bits after it of a GUTI that is written as `prefix`, five or six
/// digits and 14 hexadecimal digits.
fn written(member: &'static str, prefix: &str, text: &str) -> Result<(Plmn, u64), Error> {
    let expected = match prefix {
        "5g-guti-" => "5g-guti-, the digits of the PLMN and 14 hexadecimal digits",
        _ => "4g-guti-, the digits of the PLMN and 14 hexadecimal digits",
    };
    let form = Error::new(member, Fault::Form { expected });
    let rest = text.strip_prefix(prefix).ok_or(form)?;
    if !rest.is_ascii() || !(19..=20).contains(&rest.len()) {
        return Err(form);
    }
    let (plmn, bits) = rest.split_at(rest.len() - 14);
    let plmn = Plmn::new(&plmn[..3], &plmn[3..])?;
    let plain = bits.bytes().all(|digit| digit.is_ascii_hexdigit());
    let bits = u64::from_str_radix(bits, 16).ok().filter(|_| plain);
    Ok((plmn, bits.ok_or(form)?))
}

impl FromStr for Guti5g {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let (plmn, bits) = written("5g-guti", "5g-guti-", text)?;
        let s_tmsi = STmsi5g::from_u64(bits & ((1 << 48) - 1))?;
        Ok(Self::of(plmn, (bits >> 48) as u8, s_tmsi))
    }
}

/// An S-TMSI of EPS: the MME Code and the M-TMSI (3GPP TS 23.003 clause 2.8.1).
///
/// ```
/// use oxirush_utils::identity::STmsiEps;
///
/// let s_tmsi = STmsiEps::new(0x01, 0xdead_beef);
/// assert_eq!(s_tmsi.to_u64(), 0x01_dead_beef);
/// assert_eq!(s_tmsi.to_string(), "s-tmsi-01deadbeef");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct STmsiEps {
    mme_code: u8,
    m_tmsi: u32,
}

impl STmsiEps {
    /// The S-TMSI of its parts.
    pub fn new(mme_code: u8, m_tmsi: u32) -> Self {
        Self { mme_code, m_tmsi }
    }

    /// The MME Code.
    pub fn mme_code(&self) -> u8 {
        self.mme_code
    }

    /// The M-TMSI.
    pub fn m_tmsi(&self) -> u32 {
        self.m_tmsi
    }

    /// The 40 bits of the S-TMSI as a number: the MME Code, then the M-TMSI.
    pub fn to_u64(&self) -> u64 {
        u64::from(self.mme_code) << 32 | u64::from(self.m_tmsi)
    }
}

impl fmt::Display for STmsiEps {
    /// `s-tmsi-` and the 40 bits in hexadecimal.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "s-tmsi-{:010x}", self.to_u64())
    }
}

impl fmt::Debug for STmsiEps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "STmsiEps({self})")
    }
}

/// A GUTI of EPS: a PLMN, the MME Group ID of 16 bits, the MME Code of 8 and the M-TMSI
/// of 32 (3GPP TS 23.003 clause 2.8.1).
///
/// It is written and read as `4g-guti-`, the digits of the MCC and of the MNC, then
/// the MME identifier and the M-TMSI as 14 hexadecimal digits.
///
/// ```
/// use oxirush_utils::identity::{GutiEps, Plmn};
///
/// let guti = GutiEps::new(Plmn::new("208", "93")?, 0x0002, 0x01, 0xdead_beef);
/// let octets = guti.to_nas_octets();
/// assert_eq!(octets, [0xf6, 0x02, 0xf8, 0x39, 0x00, 0x02, 0x01, 0xde, 0xad, 0xbe, 0xef]);
/// assert_eq!(GutiEps::from_nas_octets(&octets)?, guti);
/// assert_eq!(guti.to_string(), "4g-guti-20893000201deadbeef");
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GutiEps {
    plmn: Plmn,
    mme_group_id: u16,
    s_tmsi: STmsiEps,
}

impl GutiEps {
    /// The GUTI of its parts.
    pub fn new(plmn: Plmn, mme_group_id: u16, mme_code: u8, m_tmsi: u32) -> Self {
        Self {
            plmn,
            mme_group_id,
            s_tmsi: STmsiEps::new(mme_code, m_tmsi),
        }
    }

    /// The PLMN.
    pub fn plmn(&self) -> Plmn {
        self.plmn
    }

    /// The MME Group ID.
    pub fn mme_group_id(&self) -> u16 {
        self.mme_group_id
    }

    /// The MME Code.
    pub fn mme_code(&self) -> u8 {
        self.s_tmsi.mme_code
    }

    /// The M-TMSI.
    pub fn m_tmsi(&self) -> u32 {
        self.s_tmsi.m_tmsi
    }

    /// The S-TMSI: the GUTI without its PLMN and its MME Group ID.
    pub fn s_tmsi(&self) -> STmsiEps {
        self.s_tmsi
    }

    /// The 11 octets of the GUTI as an EPS mobile identity: `f6`, the PLMN in the order
    /// of NAS, the MME Group ID, the MME Code, then the M-TMSI (3GPP TS 24.301 clause
    /// 9.9.3.12, figure 9.9.3.12.1).
    pub fn to_nas_octets(&self) -> [u8; 11] {
        let [p1, p2, p3] = self.plmn.to_nas_octets();
        let [g1, g2] = self.mme_group_id.to_be_bytes();
        let [t1, t2, t3, t4] = self.s_tmsi.m_tmsi.to_be_bytes();
        [
            0xf6,
            p1,
            p2,
            p3,
            g1,
            g2,
            self.s_tmsi.mme_code,
            t1,
            t2,
            t3,
            t4,
        ]
    }

    /// The GUTI that `octets` hold as an EPS mobile identity.
    ///
    /// # Errors
    ///
    /// `octets` are not 11, the first is not `f6`, or the three after it are no PLMN.
    /// Bit 4 of the first octet is the odd/even indication, which is 0 for a GUTI: no
    /// tolerant reading of it is known to be needed.
    pub fn from_nas_octets(octets: &[u8]) -> Result<Self, Error> {
        let [first, p1, p2, p3, g1, g2, mme_code, tmsi @ ..] = sized::<11>("guti", octets)?;
        if first != 0xf6 {
            let fault = Fault::Octet {
                given: first,
                expected: 0xf6,
            };
            return Err(Error::new("guti", fault));
        }
        let plmn = Plmn::from_nas_octets([p1, p2, p3])?;
        let mme_group_id = u16::from_be_bytes([g1, g2]);
        Ok(Self::new(
            plmn,
            mme_group_id,
            mme_code,
            u32::from_be_bytes(tmsi),
        ))
    }
}

impl fmt::Display for GutiEps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rest = u64::from(self.mme_group_id) << 40 | self.s_tmsi.to_u64();
        write!(f, "4g-guti-{}{rest:014x}", self.plmn.digits())
    }
}

impl fmt::Debug for GutiEps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GutiEps({self})")
    }
}

impl FromStr for GutiEps {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let (plmn, bits) = written("guti", "4g-guti-", text)?;
        Ok(Self::new(
            plmn,
            (bits >> 40) as u16,
            (bits >> 32) as u8,
            bits as u32,
        ))
    }
}
