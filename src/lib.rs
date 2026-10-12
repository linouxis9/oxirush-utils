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

//! What the OxiRush crates share: the identities of 3GPP networks as values.
//!
//! ```
//! use oxirush_utils::identity::Plmn;
//!
//! let plmn: Plmn = "310-410".parse()?;
//!
//! // The three octets of a PLMN in a NAS message, and in an NGAP or S1AP message
//! assert_eq!(plmn.to_nas_octets(), [0x13, 0x00, 0x14]);
//! assert_eq!(plmn.to_ap_octets(), [0x13, 0x40, 0x01]);
//! # Ok::<(), oxirush_utils::identity::Error>(())
//! ```
//!
//! | Module | Has |
//! |--------|-----|
//! | [`identity`] | The PLMN with its two codings, TBCD digits, IMSI, IMEI, IMEISV, the GUTI and the S-TMSI of 5GS and of EPS, the tracking area identities, the types of identity |
//!
//! The crate is `no_std` with `alloc`, has no dependency without a feature, and no
//! `unsafe`.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

pub mod identity;

/// The blocks of the README, which are compiled and run with the documentation tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct _Readme;
