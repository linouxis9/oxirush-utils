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

//! The figures of the Performance section of the README, on one core:
//!
//! ```text
//! cargo run --release --manifest-path benches/Cargo.toml
//! ```
//!
//! For each operation, the time of a call in the fastest of three runs, and the
//! allocations and the octets that a call asks for: a tree of the crate against a
//! `serde_json::Value` on the same JSON, and the identities read from text and from
//! octets.

use oxirush_utils::identity::{Guti5g, Imsi, ImsiSuci, Plmn};
use oxirush_utils::tree::{self, Map, Value};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

/// An allocator that counts, for the allocations of one call.
struct Counting;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static OCTETS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        OCTETS.fetch_add(layout.size(), Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        OCTETS.fetch_add(size, Relaxed);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// A call's time in ns, in the fastest of three runs of `iterations` calls, and its
/// allocations and octets.
fn measure(iterations: u32, mut call: impl FnMut()) -> (f64, usize, usize) {
    for _ in 0..iterations {
        call();
    }
    let before = (ALLOCATIONS.load(Relaxed), OCTETS.load(Relaxed));
    call();
    let counted = (
        ALLOCATIONS.load(Relaxed) - before.0,
        OCTETS.load(Relaxed) - before.1,
    );
    let mut fastest = f64::MAX;
    for _ in 0..3 {
        let started = Instant::now();
        for _ in 0..iterations {
            call();
        }
        fastest = fastest.min(started.elapsed().as_secs_f64() * 1e9 / f64::from(iterations));
    }
    (fastest, counted.0, counted.1)
}

/// A number with a comma between each three digits.
fn grouped(number: u64) -> String {
    let digits = number.to_string();
    let mut grouped = String::new();
    for (position, digit) in digits.chars().enumerate() {
        if position > 0 && (digits.len() - position).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// A row of the table of the README: what the crate takes, what serde_json takes, and
/// how many times the time of the crate that is.
fn row(name: &str, ours: (f64, usize, usize), theirs: Option<(f64, usize, usize)>) {
    let shown = |(ns, allocations, octets): (f64, usize, usize)| {
        let time = match ns < 1e6 {
            true => format!("{} ns", grouped(ns.round() as u64)),
            false => format!("{:.1} ms", ns / 1e6),
        };
        format!(
            "{time}, {} allocations of {} octets",
            grouped(allocations as u64),
            grouped(octets as u64)
        )
    };
    match theirs {
        Some(theirs) => println!(
            "| {name} | {} | {} | {:.1}x |",
            shown(ours),
            shown(theirs),
            theirs.0 / ours.0
        ),
        None => println!("| {name} | {} | | |", shown(ours)),
    }
}

fn main() {
    // The tree of an NGAP INITIAL CONTEXT SETUP REQUEST, as oxirush-ngap writes it.
    let message = r#"{"message":"InitialContextSetupRequest","ies":[{"AMF-UE-NGAP-ID":3},{"RAN-UE-NGAP-ID":2},{"GUAMI":{"pLMNIdentity":"208-93","aMFRegionID":202,"aMFSetID":1020,"aMFPointer":1}},{"PDUSessionResourceSetupListCxtReq":[{"pDUSessionID":1,"s-NSSAI":{"sST":1,"sD":"0000ff"},"pDUSessionResourceSetupRequestTransfer":{"PDUSessionAggregateMaximumBitRate":{"pDUSessionAggregateMaximumBitRateDL":1000000000,"pDUSessionAggregateMaximumBitRateUL":1000000000}}}]},{"AllowedNSSAI":[{"s-NSSAI":{"sST":1,"sD":"0000ff"}}]},{"UESecurityCapabilities":{"nRencryptionAlgorithms":{"value":"e000","length":16},"nRintegrityProtectionAlgorithms":{"value":"e000","length":16},"eUTRAencryptionAlgorithms":{"value":"e000","length":16},"eUTRAintegrityProtectionAlgorithms":{"value":"e000","length":16}}},{"SecurityKey":"5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"},{"NAS-PDU":"7e02a3b4c5d6017e0042010177000bf202f8394001ca00006cf15400"}]}"#;
    println!("| Operation | this crate | serde_json 1 | serde_json takes |");
    println!("|---|---|---|---|");
    let ours = measure(200_000, || {
        black_box(black_box(message).parse::<Value>().unwrap());
    });
    let theirs = measure(200_000, || {
        black_box(serde_json::from_str::<serde_json::Value>(black_box(message)).unwrap());
    });
    row(
        &format!("Read a tree of {} octets", message.len()),
        ours,
        Some(theirs),
    );
    let read: Value = message.parse().unwrap();
    let json: serde_json::Value = serde_json::from_str(message).unwrap();
    let ours = measure(200_000, || {
        black_box(black_box(&read).to_string());
    });
    let theirs = measure(200_000, || {
        black_box(black_box(&json).to_string());
    });
    row("Write it", ours, Some(theirs));
    let ours = measure(200_000, || {
        black_box(tree::select(black_box(&read), "/ies/2/GUAMI/aMFSetID").unwrap());
    });
    let theirs = measure(200_000, || {
        black_box(black_box(&json).pointer("/ies/2/GUAMI/aMFSetID").unwrap());
    });
    row("Select a value of it", ours, Some(theirs));

    // An object of 100,000 members, put one after the other and read as text.
    let names: Vec<String> = (0..100_000).map(|index| format!("m{index}")).collect();
    let ours = measure(3, || {
        let mut map = Map::new();
        for name in &names {
            map.insert(name.clone(), Value::Null);
        }
        black_box(map);
    });
    let theirs = measure(3, || {
        let mut map = serde_json::Map::new();
        for name in &names {
            map.insert(name.clone(), serde_json::Value::Null);
        }
        black_box(map);
    });
    row("Put 100,000 members into an object", ours, Some(theirs));
    let members: Vec<String> = names.iter().map(|name| format!("\"{name}\":0")).collect();
    let wide = format!("{{{}}}", members.join(","));
    let ours = measure(3, || {
        black_box(black_box(&wide).parse::<Value>().unwrap());
    });
    let theirs = measure(3, || {
        black_box(serde_json::from_str::<serde_json::Value>(black_box(&wide)).unwrap());
    });
    row(
        &format!(
            "Read an object of 100,000 members, {} octets",
            grouped(wide.len() as u64)
        ),
        ours,
        Some(theirs),
    );

    // The identities.
    let plmn = Plmn::new("208", "93").unwrap();
    row(
        "Read a PLMN from text",
        measure(2_000_000, || {
            black_box(black_box("208-93").parse::<Plmn>().unwrap());
        }),
        None,
    );
    row(
        "Read an IMSI from text",
        measure(2_000_000, || {
            black_box(Imsi::new(black_box("208930000000001")).unwrap());
        }),
        None,
    );
    let guti = [
        0xf2, 0x02, 0xf8, 0x39, 0x02, 0x00, 0x40, 0xc0, 0x00, 0x06, 0xcf,
    ];
    row(
        "Read a 5G-GUTI from a 5GS mobile identity",
        measure(2_000_000, || {
            black_box(Guti5g::from_nas_octets(black_box(&guti)).unwrap());
        }),
        None,
    );
    let suci = [
        0x01, 0x02, 0xf8, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
    ];
    row(
        "Read the SUCI of an IMSI from a 5GS mobile identity",
        measure(2_000_000, || {
            black_box(ImsiSuci::from_nas_octets(black_box(&suci)).unwrap());
        }),
        None,
    );
    row(
        "Write a PLMN in the order of NGAP",
        measure(2_000_000, || {
            black_box(black_box(plmn).to_ap_octets());
        }),
        None,
    );
}
