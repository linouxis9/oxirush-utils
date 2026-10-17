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

//! The identities of 3GPP networks as values: what every family of messages holds and
//! prints the same way.
//!
//! A value is checked when it is made, so it always encodes. It prints whole, and never
//! masked: `Display` is the form that a log and a tree show, and `Debug` the same with
//! the name of the type.
//!
//! What holds a text or a list needs the feature `alloc`: the SUPI that owns its text,
//! the SUCI of an IMSI and the SUCI in the form of a NAI, TBCD strings, and identity
//! digits as octets. The rest needs no allocator: the PLMN, the IMSI, the IMEI and the
//! IMEISV as values, the GUTIs, the S-TMSIs and the TAIs, the types of identity, the
//! routing indicator and the protection scheme of a SUCI, the grammar of a NAI, a SUPI
//! that borrows its text, and the errors.
//!
//! ```
//! use oxirush_utils::identity::{Guti5g, Imsi, Plmn};
//!
//! let plmn: Plmn = "208-93".parse()?;
//! let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;
//! println!("{plmn} {guti} {}", Imsi::new("208930000000001")?);
//! # Ok::<(), oxirush_utils::identity::Error>(())
//! ```
//!
//! ```text
//! 208-93 5g-guti-20893caff81deadbeef 208930000000001
//! ```
//!
//! # Octets
//!
//! `from_nas_octets` and its kin are canonical: they give a value only for the octets
//! that the value is written as, so what is read encodes back to what was received, and
//! anything else is an [`Error`] that names the member and says why.
//!
//! The readers named `_tolerant` take what other implementations are known to send or
//! to accept, and nothing beyond it: they give the value with a list of what a sender by
//! the clause would have written otherwise. Each tolerance is listed where it is read,
//! with its clause and its evidence:
//!
//! | Reader | Reads also | Why |
//! |--------|-----------|-----|
#![cfg_attr(
    feature = "alloc",
    doc = "| [`digits::decode_tolerant`], and the `_tolerant` readers of [`Imsi`], [`Imei`] and [`Imeisv`] | another half octet than `1111` as the end mark of an even number of digits | TS 24.008 clause 10.5.1.4; PacketRusher sends an IMEISV so, and Open5GS, free5GC and pycrate read it |"
)]
#![cfg_attr(
    not(feature = "alloc"),
    doc = "| `digits::decode_tolerant`, and the `_tolerant` readers of `Imsi`, `Imei` and `Imeisv` | another half octet than `1111` as the end mark of an even number of digits | TS 24.008 clause 10.5.1.4; PacketRusher sends an IMEISV so, and Open5GS, free5GC and pycrate read it |"
)]
//! | [`Guti5g::from_nas_octets_tolerant`], [`STmsi5g::from_nas_octets_tolerant`] | the spare bit 4 of the first octet as 1 | TS 24.007 clause 11.1.4: a receiver accepts a spare bit as 0 or as 1 |
#![cfg_attr(
    feature = "alloc",
    doc = "| [`ImsiSuci::from_nas_octets_tolerant`] | a spare bit of the first octet or of the octet of the scheme as 1; a SUPI format of 4 to 7 as that of an IMSI; a scheme output of another length than its scheme gives it | TS 24.007 clause 11.1.4, and Open5GS reads neither bit 4 of the first octet nor the spare half of the scheme; TS 24.501 table 9.11.3.4.1: \"All other values are interpreted as IMSI\", as free5GC reads them; the scheme output is opaque to all but the home network, whose de-concealment refuses what is not of its profile (TS 33.501 annex C.3.3, TS 33.514 clause 4.2.1.3) |"
)]
#![cfg_attr(
    not(feature = "alloc"),
    doc = "| `ImsiSuci::from_nas_octets_tolerant` | a spare bit of the first octet or of the octet of the scheme as 1; a SUPI format of 4 to 7 as that of an IMSI; a scheme output of another length than its scheme gives it | TS 24.007 clause 11.1.4, and Open5GS reads neither bit 4 of the first octet nor the spare half of the scheme; TS 24.501 table 9.11.3.4.1: \"All other values are interpreted as IMSI\", as free5GC reads them; the scheme output is opaque to all but the home network, whose de-concealment refuses what is not of its profile (TS 33.501 annex C.3.3, TS 33.514 clause 4.2.1.3) |"
)]
#![cfg_attr(
    feature = "alloc",
    doc = "| [`NaiSuci::from_nas_octets_tolerant`] | a spare bit of the first octet as 1 | TS 24.007 clause 11.1.4 |"
)]
#![cfg_attr(
    not(feature = "alloc"),
    doc = "| `NaiSuci::from_nas_octets_tolerant` | a spare bit of the first octet as 1 | TS 24.007 clause 11.1.4 |"
)]
//!
//! Which reader a family calls is its own decision. A tolerant reader never makes a
//! value up: octets that are not the identity however they are read stay an error.

pub mod digits;
mod error;
mod guti;
pub mod nai;
mod plmn;
mod suci;
mod supi;
mod tai;
mod types;

#[cfg(feature = "alloc")]
pub use digits::tbcd;
pub use digits::{Imei, Imeisv, Imsi};
pub use error::{Error, Fault, Faults};
pub use guti::{Guti5g, GutiEps, STmsi5g, STmsiEps};
#[cfg(feature = "alloc")]
pub use nai::NaiSuci;
pub use plmn::Plmn;
#[cfg(feature = "alloc")]
pub use suci::ImsiSuci;
pub use suci::{ProtectionScheme, RoutingIndicator};
pub use supi::{Supi, SupiType};
pub use tai::{Tai5gs, TaiEps};
pub use types::{IdentityType, MobileIdentityType5gs, MobileIdentityTypeEps};

/// What a tolerant reader gives: the value, and each thing that a sender by the clause
/// would have written otherwise, as the error that a canonical reader gives for it.
/// The list is empty for octets that are canonical.
pub type Tolerated<T> = (T, Faults);
