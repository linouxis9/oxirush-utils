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

//! The types of identity of a mobile identity, by the number that each clause gives them.

use super::{Error, Fault};

macro_rules! type_of_identity {
    (
        $(#[$doc:meta])*
        $name:ident, $member:literal { $($(#[$variant_doc:meta])* $variant:ident = $bits:literal,)* }
    ) => {
        $(#[$doc])*
        #[non_exhaustive]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u8)]
        pub enum $name {
            $($(#[$variant_doc])* $variant = $bits,)*
        }

        impl $name {
            /// The three bits of the type of identity.
            pub fn bits(self) -> u8 {
                self as u8
            }

            /// The type of identity of the first octet of a mobile identity: its bits 3
            /// to 1.
            ///
            /// # Errors
            ///
            /// The bits are no type of identity of the clause.
            pub fn of(first_octet: u8) -> Result<Self, Error> {
                Self::try_from(first_octet & 0x07)
            }
        }

        impl TryFrom<u8> for $name {
            type Error = Error;

            /// The type of identity of its three bits, as a number.
            fn try_from(bits: u8) -> Result<Self, Error> {
                match bits {
                    $($bits => Ok(Self::$variant),)*
                    given => {
                        let expected = concat!("a type of identity of ", $member);
                        let given = given.into();
                        Err(Error::new("type-of-identity", Fault::Value { given, expected }))
                    }
                }
            }
        }

        impl From<$name> for u8 {
            fn from(kind: $name) -> u8 {
                kind as u8
            }
        }
    };
}

type_of_identity! {
    /// The type of identity of a 5GS mobile identity (3GPP TS 24.501 clause 9.11.3.4,
    /// table 9.11.3.4.1).
    ///
    /// ```
    /// use oxirush_utils::identity::MobileIdentityType5gs;
    ///
    /// // The first octet of a 5G-GUTI
    /// assert_eq!(MobileIdentityType5gs::of(0xf2)?, MobileIdentityType5gs::Guti);
    /// assert_eq!(MobileIdentityType5gs::Imeisv.bits(), 5);
    /// # Ok::<(), oxirush_utils::identity::Error>(())
    /// ```
    MobileIdentityType5gs, "the 5GS mobile identity" {
        /// No identity.
        NoIdentity = 0,
        /// SUCI.
        Suci = 1,
        /// 5G-GUTI.
        Guti = 2,
        /// IMEI.
        Imei = 3,
        /// 5G-S-TMSI.
        STmsi = 4,
        /// IMEISV.
        Imeisv = 5,
        /// MAC address.
        MacAddress = 6,
        /// EUI-64.
        Eui64 = 7,
    }
}

type_of_identity! {
    /// The type of identity of an EPS mobile identity (3GPP TS 24.301 clause 9.9.3.12,
    /// table 9.9.3.12.1). The other values are reserved.
    ///
    /// ```
    /// use oxirush_utils::identity::MobileIdentityTypeEps;
    ///
    /// // The first octet of a GUTI
    /// assert_eq!(MobileIdentityTypeEps::of(0xf6)?, MobileIdentityTypeEps::Guti);
    /// assert!(MobileIdentityTypeEps::try_from(2).is_err());
    /// # Ok::<(), oxirush_utils::identity::Error>(())
    /// ```
    MobileIdentityTypeEps, "the EPS mobile identity" {
        /// IMSI.
        Imsi = 1,
        /// IMEI.
        Imei = 3,
        /// GUTI.
        Guti = 6,
    }
}

type_of_identity! {
    /// The type of identity of a mobile identity of 3GPP TS 24.008 (clause 10.5.1.4,
    /// table 10.5.4), which the identity request of EPS asks for by the same numbers.
    /// The other values are reserved.
    ///
    /// ```
    /// use oxirush_utils::identity::IdentityType;
    ///
    /// assert_eq!(IdentityType::try_from(3)?, IdentityType::Imeisv);
    /// # Ok::<(), oxirush_utils::identity::Error>(())
    /// ```
    IdentityType, "the mobile identity of TS 24.008" {
        /// No identity.
        NoIdentity = 0,
        /// IMSI.
        Imsi = 1,
        /// IMEI.
        Imei = 2,
        /// IMEISV.
        Imeisv = 3,
        /// TMSI, P-TMSI or M-TMSI.
        Tmsi = 4,
        /// TMGI and optional MBMS session identity.
        Tmgi = 5,
    }
}
