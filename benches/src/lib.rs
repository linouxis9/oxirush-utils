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

//! What the benchmarks of oxirush-utils measure: the operations of the table
//! of the README, each a call on its input. The two programs of `benches/`
//! share them:
//!
//! - `tree` times each call with criterion, and profiles it with pprof;
//! - `allocations` counts the allocations of one call of each.
//!
//! A tree of the crate is measured beside a `serde_json::Value` of the default
//! features on the same JSON, and the identities alone. In this directory:
//!
//! ```sh
//! cargo bench --bench tree                      # the time, with criterion
//! cargo bench --bench tree -- read-tree         # those whose name has it
//! cargo bench --bench allocations               # the allocations of a call
//! cargo bench --bench tree -- --profile-time 10 read-tree/oxirush-utils
//! go tool pprof -top target/criterion/read-tree/oxirush-utils/profile/profile.pb
//! ```
//!
//! `--profile-time` runs the benchmark under [pprof](https://crates.io/crates/pprof)
//! and writes a CPU profile that `go tool pprof` reads; with `PROFILE=flamegraph`
//! in the environment it writes `flamegraph.svg` beside it instead.

use oxirush_utils::identity::{Guti5g, Imsi, ImsiSuci, Plmn};
use oxirush_utils::tree::{self, Map, Value};
use std::hint::black_box;

/// One operation of one implementation: the row of the README is its
/// `group`, and `implementation` is `oxirush-utils` or `serde_json`.
pub struct Operation {
    pub group: &'static str,
    pub implementation: &'static str,
    /// A call of a few milliseconds, which criterion samples fewer times.
    pub long: bool,
    pub call: Box<dyn FnMut()>,
}

/// The tree of an NGAP INITIAL CONTEXT SETUP REQUEST, as oxirush-ngap writes it.
pub const MESSAGE: &str = r#"{"message":"InitialContextSetupRequest","ies":[{"AMF-UE-NGAP-ID":3},{"RAN-UE-NGAP-ID":2},{"GUAMI":{"pLMNIdentity":"208-93","aMFRegionID":202,"aMFSetID":1020,"aMFPointer":1}},{"PDUSessionResourceSetupListCxtReq":[{"pDUSessionID":1,"s-NSSAI":{"sST":1,"sD":"0000ff"},"pDUSessionResourceSetupRequestTransfer":{"PDUSessionAggregateMaximumBitRate":{"pDUSessionAggregateMaximumBitRateDL":1000000000,"pDUSessionAggregateMaximumBitRateUL":1000000000}}}]},{"AllowedNSSAI":[{"s-NSSAI":{"sST":1,"sD":"0000ff"}}]},{"UESecurityCapabilities":{"nRencryptionAlgorithms":{"value":"e000","length":16},"nRintegrityProtectionAlgorithms":{"value":"e000","length":16},"eUTRAencryptionAlgorithms":{"value":"e000","length":16},"eUTRAintegrityProtectionAlgorithms":{"value":"e000","length":16}}},{"SecurityKey":"5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"},{"NAS-PDU":"7e02a3b4c5d6017e0042010177000bf202f8394001ca00006cf15400"}]}"#;

/// The members of the wide object: 100,000 names.
pub const MEMBERS: usize = 100_000;

/// What `REGISTRATION ACCEPT` of Open5GS had as its 5G-GUTI.
pub const GUTI: [u8; 11] = [
    0xf2, 0x02, 0xf8, 0x39, 0x02, 0x00, 0x40, 0xc0, 0x00, 0x06, 0xcf,
];

/// What PacketRusher sent as its SUCI, under the null scheme.
pub const SUCI: [u8; 13] = [
    0x01, 0x02, 0xf8, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

/// The operations, in the order of the table of the README.
pub fn operations() -> Vec<Operation> {
    let mut operations = Vec::new();
    let mut add = |group, implementation, long, call: Box<dyn FnMut()>| {
        operations.push(Operation {
            group,
            implementation,
            long,
            call,
        })
    };

    // A tree of a message: read, written, and a value of it selected.
    add(
        "read-tree",
        "oxirush-utils",
        false,
        Box::new(|| {
            black_box(black_box(MESSAGE).parse::<Value>().unwrap());
        }),
    );
    add(
        "read-tree",
        "serde_json",
        false,
        Box::new(|| {
            black_box(serde_json::from_str::<serde_json::Value>(black_box(MESSAGE)).unwrap());
        }),
    );
    let read: Value = MESSAGE.parse().unwrap();
    let json: serde_json::Value = serde_json::from_str(MESSAGE).unwrap();
    let (tree, value) = (read.clone(), json.clone());
    add(
        "write-tree",
        "oxirush-utils",
        false,
        Box::new(move || {
            black_box(black_box(&tree).to_string());
        }),
    );
    add(
        "write-tree",
        "serde_json",
        false,
        Box::new(move || {
            black_box(black_box(&value).to_string());
        }),
    );
    add(
        "select",
        "oxirush-utils",
        false,
        Box::new(move || {
            black_box(tree::select(black_box(&read), "/ies/2/GUAMI/aMFSetID").unwrap());
        }),
    );
    add(
        "select",
        "serde_json",
        false,
        Box::new(move || {
            black_box(black_box(&json).pointer("/ies/2/GUAMI/aMFSetID").unwrap());
        }),
    );

    // An object of 100,000 members, put one after the other and read as text.
    let names: Vec<String> = (0..MEMBERS).map(|index| format!("m{index}")).collect();
    let ours = names.clone();
    add(
        "put-100000-members",
        "oxirush-utils",
        true,
        Box::new(move || {
            let mut map = Map::new();
            for name in &ours {
                map.insert(name.clone(), Value::Null);
            }
            black_box(map);
        }),
    );
    let theirs = names.clone();
    add(
        "put-100000-members",
        "serde_json",
        true,
        Box::new(move || {
            let mut map = serde_json::Map::new();
            for name in &theirs {
                map.insert(name.clone(), serde_json::Value::Null);
            }
            black_box(map);
        }),
    );
    let wide = wide(&names);
    let ours = wide.clone();
    add(
        "read-100000-members",
        "oxirush-utils",
        true,
        Box::new(move || {
            black_box(black_box(&ours).parse::<Value>().unwrap());
        }),
    );
    add(
        "read-100000-members",
        "serde_json",
        true,
        Box::new(move || {
            black_box(serde_json::from_str::<serde_json::Value>(black_box(&wide)).unwrap());
        }),
    );

    // The identities.
    add(
        "plmn-from-text",
        "oxirush-utils",
        false,
        Box::new(|| {
            black_box(black_box("208-93").parse::<Plmn>().unwrap());
        }),
    );
    add(
        "imsi-from-text",
        "oxirush-utils",
        false,
        Box::new(|| {
            black_box(Imsi::new(black_box("208930000000001")).unwrap());
        }),
    );
    add(
        "guti-from-nas",
        "oxirush-utils",
        false,
        Box::new(|| {
            black_box(Guti5g::from_nas_octets(black_box(&GUTI)).unwrap());
        }),
    );
    add(
        "suci-from-nas",
        "oxirush-utils",
        false,
        Box::new(|| {
            black_box(ImsiSuci::from_nas_octets(black_box(&SUCI)).unwrap());
        }),
    );
    let plmn = Plmn::new("208", "93").unwrap();
    add(
        "plmn-to-ap",
        "oxirush-utils",
        false,
        Box::new(move || {
            black_box(black_box(plmn).to_ap_octets());
        }),
    );
    operations
}

/// The text of an object whose members are `names`, each with the value 0.
pub fn wide(names: &[String]) -> String {
    let members: Vec<String> = names.iter().map(|name| format!("\"{name}\":0")).collect();
    format!("{{{}}}", members.join(","))
}
