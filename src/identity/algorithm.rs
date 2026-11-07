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

//! The algorithm identity of NAS and AS security, which 5GS and EPS number alike.

/// A security algorithm by its identity: the null algorithm, SNOW 3G, AES or ZUC (3GPP
/// TS 33.501 clause 5.11.1, TS 33.401 clause 5.1.3).
///
/// Ciphering and integrity number their algorithms alike, in 5GS and in EPS: 128-NEA2,
/// 128-NIA2, 128-EEA2 and 128-EIA2 are all AES, the identity 2. A NAS message carries the
/// identity in three bits, and a number that is no algorithm here is given back as it
/// is: 4 to 7 are reserved. The enum is exhaustive, so that a match on it handles each
/// algorithm: one that a later release defines comes with a new version of the crate.
///
/// ```
/// use oxirush_utils::identity::Algorithm;
///
/// assert_eq!(Algorithm::try_from(2), Ok(Algorithm::Aes));
/// assert_eq!(u8::from(Algorithm::Zuc), 3);
/// assert_eq!(Algorithm::try_from(4), Err(4));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Algorithm {
    /// The null algorithm, the identity 0: NEA0 and NIA0, EEA0 and EIA0. Its integrity is
    /// for unauthenticated emergency sessions only (TS 33.501 clause 5.5.2).
    Null,
    /// SNOW 3G, the identity 1: 128-NEA1 and 128-NIA1, 128-EEA1 and 128-EIA1.
    Snow3g,
    /// AES, the identity 2: AES-CTR for ciphering and AES-CMAC for integrity.
    Aes,
    /// ZUC, the identity 3.
    Zuc,
}

impl TryFrom<u8> for Algorithm {
    /// The number, which is no algorithm here.
    type Error = u8;

    fn try_from(identity: u8) -> Result<Self, u8> {
        match identity {
            0 => Ok(Self::Null),
            1 => Ok(Self::Snow3g),
            2 => Ok(Self::Aes),
            3 => Ok(Self::Zuc),
            other => Err(other),
        }
    }
}

impl From<Algorithm> for u8 {
    fn from(algorithm: Algorithm) -> u8 {
        match algorithm {
            Algorithm::Null => 0,
            Algorithm::Snow3g => 1,
            Algorithm::Aes => 2,
            Algorithm::Zuc => 3,
        }
    }
}
