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

//! The tracking area identities of both systems.

use super::{Error, Fault, Plmn};
use core::fmt;

/// A tracking area identity of 5GS: a PLMN and a tracking area code of 24 bits (3GPP
/// TS 23.003 clause 28.6).
///
/// ```
/// use oxirush_utils::identity::{Plmn, Tai5gs};
///
/// let tai = Tai5gs::new(Plmn::new("208", "93")?, 1)?;
/// assert_eq!(tai.to_nas_octets(), [0x02, 0xf8, 0x39, 0x00, 0x00, 0x01]);
/// assert_eq!(tai.to_string(), "208-93-000001");
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tai5gs {
    plmn: Plmn,
    tac: u32,
}

impl Tai5gs {
    /// The tracking area identity of a PLMN and a tracking area code.
    ///
    /// # Errors
    ///
    /// `tac` has more than 24 bits.
    pub fn new(plmn: Plmn, tac: u32) -> Result<Self, Error> {
        if tac > 0xff_ffff {
            let fault = Fault::Range {
                given: tac.into(),
                max: 0xff_ffff,
            };
            return Err(Error::new("tac", fault));
        }
        Ok(Self { plmn, tac })
    }

    /// The PLMN.
    pub fn plmn(&self) -> Plmn {
        self.plmn
    }

    /// The tracking area code: 24 bits.
    pub fn tac(&self) -> u32 {
        self.tac
    }

    /// The six octets of the value of a 5GS tracking area identity: the PLMN in the
    /// order of NAS, then the tracking area code (3GPP TS 24.501 clause 9.11.3.8).
    pub fn to_nas_octets(&self) -> [u8; 6] {
        let [p1, p2, p3] = self.plmn.to_nas_octets();
        let [_, t1, t2, t3] = self.tac.to_be_bytes();
        [p1, p2, p3, t1, t2, t3]
    }

    /// The tracking area identity that `octets` hold.
    ///
    /// # Errors
    ///
    /// The first three octets are no PLMN: TS 24.501 clause 9.11.3.8 has a receiver
    /// treat a tracking area identity with such an MCC or MNC as deleted.
    pub fn from_nas_octets(octets: [u8; 6]) -> Result<Self, Error> {
        let [p1, p2, p3, t1, t2, t3] = octets;
        let plmn = Plmn::from_nas_octets([p1, p2, p3])?;
        Ok(Self {
            plmn,
            tac: u32::from_be_bytes([0, t1, t2, t3]),
        })
    }
}

impl fmt::Display for Tai5gs {
    /// The PLMN, a hyphen and the tracking area code as six hexadecimal digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{:06x}", self.plmn, self.tac)
    }
}

impl fmt::Debug for Tai5gs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tai5gs({self})")
    }
}

/// A tracking area identity of EPS: a PLMN and a tracking area code of 16 bits (3GPP
/// TS 23.003 clause 19.4.2.3).
///
/// ```
/// use oxirush_utils::identity::{Plmn, TaiEps};
///
/// let tai = TaiEps::new(Plmn::new("208", "93")?, 1);
/// assert_eq!(tai.to_nas_octets(), [0x02, 0xf8, 0x39, 0x00, 0x01]);
/// assert_eq!(tai.to_string(), "208-93-0001");
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaiEps {
    plmn: Plmn,
    tac: u16,
}

impl TaiEps {
    /// The tracking area identity of a PLMN and a tracking area code.
    pub fn new(plmn: Plmn, tac: u16) -> Self {
        Self { plmn, tac }
    }

    /// The PLMN.
    pub fn plmn(&self) -> Plmn {
        self.plmn
    }

    /// The tracking area code.
    pub fn tac(&self) -> u16 {
        self.tac
    }

    /// The five octets of the value of a tracking area identity of EPS: the PLMN in the
    /// order of NAS, then the tracking area code (3GPP TS 24.301 clause 9.9.3.32).
    pub fn to_nas_octets(&self) -> [u8; 5] {
        let [p1, p2, p3] = self.plmn.to_nas_octets();
        let [t1, t2] = self.tac.to_be_bytes();
        [p1, p2, p3, t1, t2]
    }

    /// The tracking area identity that `octets` hold.
    ///
    /// # Errors
    ///
    /// The first three octets are no PLMN.
    pub fn from_nas_octets(octets: [u8; 5]) -> Result<Self, Error> {
        let [p1, p2, p3, t1, t2] = octets;
        let plmn = Plmn::from_nas_octets([p1, p2, p3])?;
        Ok(Self::new(plmn, u16::from_be_bytes([t1, t2])))
    }
}

impl fmt::Display for TaiEps {
    /// The PLMN, a hyphen and the tracking area code as four hexadecimal digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{:04x}", self.plmn, self.tac)
    }
}

impl fmt::Debug for TaiEps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TaiEps({self})")
    }
}
