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

//! What the OxiRush crates share: the identities of 3GPP networks as values, and one
//! tree with paths for the messages of every family. `no_std`, without a dependency
//! unless a feature asks for one.
//!
//! ```rust
//! use oxirush_utils::identity::{Guti5g, Plmn};
//!
//! fn main() -> Result<(), oxirush_utils::identity::Error> {
//!     let plmn: Plmn = "310-410".parse()?;
//!     let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;
//!
//!     // The three octets of a PLMN in a NAS message, and in an NGAP or S1AP message
//!     println!("{plmn}: {:02x?} in NAS, {:02x?} in NGAP", plmn.to_nas_octets(), plmn.to_ap_octets());
//!
//!     // A 5G-GUTI as text, and as a 5GS mobile identity
//!     println!("{guti}: {:02x?}", guti.to_nas_octets());
//!     Ok(())
//! }
//! ```
//!
//! ```text
//! 310-410: [13, 00, 14] in NAS, [13, 40, 01] in NGAP
//! 5g-guti-310410caff81deadbeef: [f2, 13, 00, 14, ca, ff, 81, de, ad, be, ef]
//! ```
//!
//! # Tasks
//!
//! The [README](https://github.com/linouxis9/oxirush-utils#readme) has an example of
//! each, with what it prints, except the rules of a family of messages: the
//! documentation of [`tree::Family`] has theirs.
//!
//! | Task | Items |
//! |------|-------|
//! | An identity through its forms | [`Imsi`](identity::Imsi), [`Plmn`](identity::Plmn), [`ImsiSuci`](identity::ImsiSuci), [`Supi`](identity::Supi), [`Guti5g`](identity::Guti5g), [`STmsi5g`](identity::STmsi5g) |
//! | IMEI, IMEISV, and the identities of EPS | [`Imei`](identity::Imei), [`Imeisv`](identity::Imeisv), [`GutiEps`](identity::GutiEps), [`Tai5gs`](identity::Tai5gs), [`TaiEps`](identity::TaiEps) |
//! | The SUCI in the form of a NAI | [`NaiSuci`](identity::NaiSuci), [`nai::check_realm`](identity::nai::check_realm) |
//! | What other implementations send | [`Imeisv::from_nas_octets_tolerant`](identity::Imeisv::from_nas_octets_tolerant), [`Fault`](identity::Fault) |
//! | A type as a tree | [`tree::Tree`], [`tree!`], [`tree::to_value`], [`tree::from_value`] |
//! | Paths | [`tree::select`], [`tree::paths`], [`tree::set`], [`tree::insert`], [`tree::remove`] |
//! | The rules of a family of messages | [`tree::Family`], [`tree::select_in`], [`tree::written_as`] |
//! | Any format of serde | [`impl_serde!`] |
//!
// Without its feature an item of the tasks is plain text: the links are checked in the
// build of every feature, which docs.rs makes.
#![cfg_attr(not(feature = "serde"), allow(rustdoc::broken_intra_doc_links))]
//! # Modules
//!
//! | Module | Has | Feature |
//! |--------|-----|---------|
//! | [`identity`] | The PLMN with its two codings, TBCD digits, IMSI, IMEI, IMEISV, the SUPI with its type, the SUCI of an IMSI and the SUCI in the form of a NAI with the grammar of a NAI, the GUTI and the S-TMSI of 5GS and of EPS, the tracking area identities, the types of identity | none |
// A module that a feature turns on is a link when the build has it.
#![cfg_attr(
    feature = "tree",
    doc = "| [`tree`](mod@tree) | The tree of a message as a JSON value that keeps the order of its members, and `tree!` that writes one: the forms of the plain values, the writer and the reader of members, and paths with `select`, `paths`, `set`, `remove`, `insert` | `tree` |"
)]
#![cfg_attr(
    not(feature = "tree"),
    doc = "| `tree` | The tree of a message as a JSON value that keeps the order of its members, and `tree!` that writes one: the forms of the plain values, the writer and the reader of members, and paths with `select`, `paths`, `set`, `remove`, `insert` | `tree` |"
)]
#![cfg_attr(
    feature = "serde",
    doc = "| [`tree::serde`] | `Serialize` and `Deserialize` for a type through its tree | `serde` |"
)]
#![cfg_attr(
    not(feature = "serde"),
    doc = "| `tree::serde` | `Serialize` and `Deserialize` for a type through its tree | `serde` |"
)]
//!
//! The crate is `no_std` with `alloc`, has no dependency without a feature, and no
//! `unsafe`. With the feature `idna`, the realm and the username of a NAI are also
//! checked as internationalized names (see [`identity::nai::check_realm`]).
//!
//! A tree keeps the order of the specification by itself: the crate turns on no
//! feature of `serde_json` that orders a map, and the JSON of a program that depends on
//! it keeps the order that the program chose.
//!
//! # The OxiRush family
//!
//! - [oxirush-utils](https://docs.rs/oxirush-utils): the 3GPP identities, and the JSON tree with its paths that the others share.
//! - [oxirush-aper](https://docs.rs/oxirush-aper): aligned PER, the runtime of the NGAP and S1AP codecs.
//! - [oxirush-nas](https://docs.rs/oxirush-nas): the NAS messages of 5GS and EPS.
//! - [oxirush-security](https://docs.rs/oxirush-security): NAS security, AKA and the derivation of keys.
//! - [oxirush-ngap](https://docs.rs/oxirush-ngap): NGAP, between a gNB and an AMF.
//! - [oxirush-s1ap](https://docs.rs/oxirush-s1ap): S1AP, between an eNB and an MME.
//! - [oxirush-gtp-u](https://docs.rs/oxirush-gtp-u): GTP-U tunnels, with a test UPF.
//! - [oxirush-sctp](https://docs.rs/oxirush-sctp): SCTP sockets, which carry NGAP and S1AP.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod identity;
#[cfg(feature = "tree")]
#[cfg_attr(docsrs, doc(cfg(feature = "tree")))]
pub mod tree;

/// The blocks of the README, which are compiled and run with the documentation tests.
#[cfg(all(doctest, feature = "serde"))]
#[doc = include_str!("../README.md")]
struct _Readme;
