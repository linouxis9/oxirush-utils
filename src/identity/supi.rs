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

//! The SUPI, and the types of SUPI by both of their numberings.

use super::{Error, Fault, Imsi};
use alloc::borrow::Cow;
use alloc::string::String;
use core::fmt;
use core::str::FromStr;

/// The type of a SUPI: an IMSI, a network specific identifier, a global line identifier
/// or a global cable identifier.
///
/// Two clauses number the types, and not alike: the SUPI type of a SUCI (3GPP TS 23.003
/// clause 2.2B) has the GLI as 2 and the GCI as 3, and the SUPI format of a 5GS mobile
/// identity (TS 24.501 clause 9.11.3.4) has the GCI as 2 and the GLI as 3.
///
/// ```
/// use oxirush_utils::identity::SupiType;
///
/// assert_eq!((SupiType::Gci.nas_format(), SupiType::Gci.type_number()), (2, 3));
/// assert_eq!(SupiType::from_nas_format(3)?, SupiType::Gli);
/// assert_eq!(SupiType::from_type_number(3)?, SupiType::Gci);
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SupiType {
    /// An IMSI.
    Imsi,
    /// A network specific identifier, which is a NAI.
    NetworkSpecific,
    /// A global line identifier.
    Gli,
    /// A global cable identifier.
    Gci,
}

impl SupiType {
    /// The SUPI type of 3GPP TS 23.003 clause 2.2B: 0 for an IMSI, 1 for a network
    /// specific identifier, 2 for a GLI and 3 for a GCI.
    pub fn type_number(self) -> u8 {
        match self {
            Self::Imsi => 0,
            Self::NetworkSpecific => 1,
            Self::Gli => 2,
            Self::Gci => 3,
        }
    }

    /// The type of a SUPI type of TS 23.003 clause 2.2B.
    ///
    /// # Errors
    ///
    /// The number is 4 to 7, which are spare, or above.
    pub fn from_type_number(number: u8) -> Result<Self, Error> {
        match number {
            0 => Ok(Self::Imsi),
            1 => Ok(Self::NetworkSpecific),
            2 => Ok(Self::Gli),
            3 => Ok(Self::Gci),
            given => Err(Error::new(
                "supi-type",
                Fault::Range {
                    given: given.into(),
                    max: 3,
                },
            )),
        }
    }

    /// The SUPI format of a 5GS mobile identity (3GPP TS 24.501 table 9.11.3.4.1): 0
    /// for an IMSI, 1 for a network specific identifier, 2 for a GCI and 3 for a GLI.
    pub fn nas_format(self) -> u8 {
        match self {
            Self::Imsi => 0,
            Self::NetworkSpecific => 1,
            Self::Gci => 2,
            Self::Gli => 3,
        }
    }

    /// The type of a SUPI format of TS 24.501 table 9.11.3.4.1.
    ///
    /// # Errors
    ///
    /// The format is 4 to 7, or above. The table has a receiver read 4 to 7 as an IMSI:
    /// the tolerant reader of a SUCI does, and says so.
    pub fn from_nas_format(format: u8) -> Result<Self, Error> {
        match format {
            0 => Ok(Self::Imsi),
            1 => Ok(Self::NetworkSpecific),
            2 => Ok(Self::Gci),
            3 => Ok(Self::Gli),
            given => Err(Error::new(
                "supi-format",
                Fault::Range {
                    given: given.into(),
                    max: 3,
                },
            )),
        }
    }

    /// What a SUPI of the type starts with as text: `imsi-`, `nai-`, `gli-` or `gci-`.
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Imsi => "imsi-",
            Self::NetworkSpecific => "nai-",
            Self::Gli => "gli-",
            Self::Gci => "gci-",
        }
    }
}

/// The longest NAI of a SUPI: a username and a realm of 253 octets each, and `@`.
const LONGEST_NAI: usize = 253 + 1 + 253;

/// A SUPI: an IMSI, or a NAI that is a network specific identifier, a GLI or a GCI
/// (3GPP TS 23.003 clause 2.2A).
///
/// It is written and read with its type before it, as `imsi-208930000000001` and
/// `nai-user@example.com`, which is how the service-based interfaces write one (`Supi`
/// in the OpenAPI of TS 29.571). It borrows its text, or owns it after
/// [`into_owned`](Self::into_owned): one that is borrowed is made without an allocation.
///
/// The NAI of a SUPI is a username, `@` and a realm that is not empty, of 507 octets at
/// most. Its grammar, that of RFC 7542, is not checked here.
///
/// ```
/// use oxirush_utils::identity::{Supi, SupiType};
///
/// let supi = Supi::parse("imsi-208930000000001")?;
/// assert_eq!((supi.supi_type(), supi.identity()), (SupiType::Imsi, "208930000000001"));
/// assert_eq!(supi.as_imsi().unwrap().plmn(2)?.to_string(), "208-93");
///
/// let nai = Supi::network_specific("user@example.com")?;
/// assert_eq!(nai.to_string(), "nai-user@example.com");
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Supi<'a> {
    kind: SupiType,
    identity: Cow<'a, str>,
}

impl<'a> Supi<'a> {
    /// The SUPI that is the IMSI of these digits.
    ///
    /// # Errors
    ///
    /// `imsi` is not an IMSI: 6 to 15 decimal digits.
    pub fn imsi(imsi: &'a str) -> Result<Self, Error> {
        Imsi::new(imsi)?;
        Ok(Self {
            kind: SupiType::Imsi,
            identity: Cow::Borrowed(imsi),
        })
    }

    /// The SUPI that is a network specific identifier: the NAI `nai`.
    ///
    /// # Errors
    ///
    /// `nai` is longer than 507 octets, or is not a username, `@` and a realm.
    pub fn network_specific(nai: &'a str) -> Result<Self, Error> {
        Self::nai(SupiType::NetworkSpecific, nai)
    }

    /// The SUPI that is a global line identifier, in its NAI.
    ///
    /// # Errors
    ///
    /// As [`network_specific`](Self::network_specific).
    pub fn gli(nai: &'a str) -> Result<Self, Error> {
        Self::nai(SupiType::Gli, nai)
    }

    /// The SUPI that is a global cable identifier, in its NAI.
    ///
    /// # Errors
    ///
    /// As [`network_specific`](Self::network_specific).
    pub fn gci(nai: &'a str) -> Result<Self, Error> {
        Self::nai(SupiType::Gci, nai)
    }

    fn nai(kind: SupiType, nai: &'a str) -> Result<Self, Error> {
        if nai.len() > LONGEST_NAI {
            return Err(Error::length("nai", "octets", nai.len(), 3, LONGEST_NAI));
        }
        match nai.rsplit_once('@') {
            Some((_, realm)) if !realm.is_empty() => Ok(Self {
                kind,
                identity: Cow::Borrowed(nai),
            }),
            _ => {
                let expected = "a username, @ and a realm, as user@example.com";
                Err(Error::new("nai", Fault::Form { expected }))
            }
        }
    }

    /// The SUPI that `text` writes, with its type before it: `imsi-`, `nai-`, `gli-` or
    /// `gci-`.
    ///
    /// # Errors
    ///
    /// `text` starts with none of the four, or what follows is not a SUPI of that type.
    pub fn parse(text: &'a str) -> Result<Self, Error> {
        let kinds = [
            SupiType::Imsi,
            SupiType::NetworkSpecific,
            SupiType::Gli,
            SupiType::Gci,
        ];
        for kind in kinds {
            if let Some(identity) = text.strip_prefix(kind.prefix()) {
                return match kind {
                    SupiType::Imsi => Self::imsi(identity),
                    kind => Self::nai(kind, identity),
                };
            }
        }
        let expected = "imsi-, nai-, gli- or gci-, then the identity, as imsi-208930000000001";
        Err(Error::new("supi", Fault::Form { expected }))
    }

    /// The type of the SUPI.
    pub fn supi_type(&self) -> SupiType {
        self.kind
    }

    /// The identity without its type: the digits of an IMSI, or the whole NAI. It is
    /// what the derivation of K_AMF takes as the SUPI (3GPP TS 33.501 annex A.7.0).
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The IMSI, when the SUPI is one.
    pub fn as_imsi(&self) -> Option<Imsi> {
        match self.kind {
            SupiType::Imsi => Imsi::new(&self.identity).ok(),
            _ => None,
        }
    }

    /// The same SUPI, which owns its text.
    pub fn into_owned(self) -> Supi<'static> {
        Supi {
            kind: self.kind,
            identity: Cow::Owned(self.identity.into_owned()),
        }
    }
}

impl From<Imsi> for Supi<'static> {
    fn from(imsi: Imsi) -> Self {
        Self {
            kind: SupiType::Imsi,
            identity: Cow::Owned(String::from(imsi.as_str())),
        }
    }
}

impl fmt::Display for Supi<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.kind.prefix(), self.identity)
    }
}

impl fmt::Debug for Supi<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Supi({self})")
    }
}

impl FromStr for Supi<'static> {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        Supi::parse(text).map(Supi::into_owned)
    }
}
