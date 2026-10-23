# oxirush-utils

[![Crates.io](https://img.shields.io/crates/v/oxirush-utils.svg)](https://crates.io/crates/oxirush-utils)
[![Documentation](https://docs.rs/oxirush-utils/badge.svg)](https://docs.rs/oxirush-utils)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

What the OxiRush crates share: the identities of 3GPP networks as values, and
one JSON tree with paths for the messages of every family. `no_std`, without a
dependency unless a feature asks for one.

```rust
use oxirush_utils::identity::{Guti5g, Plmn};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    let plmn: Plmn = "310-410".parse()?;
    let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;

    // The three octets of a PLMN in a NAS message, and in an NGAP or S1AP message
    println!("{plmn}: {:02x?} in NAS, {:02x?} in NGAP", plmn.to_nas_octets(), plmn.to_ap_octets());

    // A 5G-GUTI as text, and as a 5GS mobile identity
    println!("{guti}: {:02x?}", guti.to_nas_octets());
    Ok(())
}
```

```text
310-410: [13, 00, 14] in NAS, [13, 40, 01] in NGAP
5g-guti-310410caff81deadbeef: [f2, 13, 00, 14, ca, ff, 81, de, ad, be, ef]
```

The text under a block is what the block prints.

This page describes oxirush-utils 0.1.0, which is not on crates.io yet.

## Installation

```toml
[dependencies]
oxirush-utils = { version = "0.1", features = ["alloc"] }
# With the tree and its paths, which need an allocator too:
# oxirush-utils = { version = "0.1", features = ["tree"] }
# Without an allocator, the values that have their digits in place:
# oxirush-utils = "0.1"
```

| Feature | Adds | Through |
|---------|------|---------|
| `alloc` | What holds a text or a list: the SUPI that owns its text, the SUCIs, TBCD strings, identity digits as octets | |
| `tree` | The tree of a message, its forms and its paths | `serde_json` |
| `serde` | `Serialize` and `Deserialize` for a type through its tree | `serde_core` |
| `idna` | The realm and the username of a NAI checked as internationalized names too: Unicode normalization form C, and a realm that IDNA takes as it is written | `idna`, `unicode-normalization` |

No feature is on by default. `tree` turns on one feature of `serde_json`,
`float_roundtrip`: a number with a fraction is read as the nearest f64. A tree
keeps the members of an object in the order that the program wrote them.

Until 0.1.0 is on crates.io, a program takes the crate from its repository:

```toml
[dependencies]
oxirush-utils = { git = "https://github.com/linouxis9/oxirush-utils", features = ["tree"] }
```

The minimum supported Rust version is 1.88.

## An identity through its forms

A value of `identity` is checked when it is made, and prints whole, in the
text that the service based interfaces write. An MNC has two or three
digits, and how many is part of the value: `208-93` is not `208-093`.

```rust
use oxirush_utils::identity::{Guti5g, Imsi, ImsiSuci, Plmn, RoutingIndicator, Supi};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    // An IMSI whose MNC has three digits: its PLMN and its MSIN
    let imsi = Imsi::new("310410123456789")?;
    let (plmn, msin) = (imsi.plmn(3)?, imsi.msin(3)?);
    println!("IMSI {imsi}: PLMN {plmn}, MSIN {msin}");

    // The PLMN in a NAS message (TS 24.008 clause 10.5.1.3), and in an NGAP
    // or S1AP message (TS 38.413 clause 9.3.3.5)
    println!("{:02x?} in NAS, {:02x?} in NGAP", plmn.to_nas_octets(), plmn.to_ap_octets());

    // Its SUCI under the null scheme, as text and as a 5GS mobile identity,
    // and the SUPI that the SUCI gives
    let suci = ImsiSuci::null(plmn, RoutingIndicator::default(), msin)?;
    println!("{suci}: {:02x?}", suci.to_nas_octets());
    println!("{}", Supi::from(suci.imsi().unwrap()));

    // The 5G-GUTI that an AMF assigns: AMF region, set and pointer, and the
    // 5G-TMSI. Its octets in a 5GS mobile identity, read back, and its 5G-S-TMSI
    let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;
    let octets = guti.to_nas_octets();
    println!("{guti}: {octets:02x?}");
    println!("{}", Guti5g::from_nas_octets(&octets)?.s_tmsi());

    // What is no PLMN says which member is at fault, and why
    println!("{}", Plmn::new("208", "9").unwrap_err());
    Ok(())
}
```

```text
IMSI 310410123456789: PLMN 310-410, MSIN 123456789
[13, 00, 14] in NAS, [13, 40, 01] in NGAP
suci-0-310-410-0-0-0-123456789: [01, 13, 00, 14, f0, ff, 00, 00, 21, 43, 65, 87, f9]
imsi-310410123456789
5g-guti-310410caff81deadbeef: [f2, 13, 00, 14, ca, ff, 81, de, ad, be, ef]
5g-s-tmsi-ff81deadbeef
`mnc` has 1 digit, and must have 2 or 3
```

## Where each form goes

The NAS octets of an identity are the value of an IE of
[oxirush-nas](https://github.com/linouxis9/oxirush-nas), and the AP octets
of a PLMN those of the NGAP and S1AP identities of
[oxirush-ngap](https://github.com/linouxis9/oxirush-ngap) and
[oxirush-s1ap](https://github.com/linouxis9/oxirush-s1ap). Their messages
are trees of this crate, read by the same paths.

```rust,ignore
use oxirush_nas::nas_5gs::{
    Nas5gsMessage, NasFGsMobileIdentity, NasFGsRegistrationResult, NasRegistrationAccept,
    RegistrationResult,
};
use oxirush_ngap::{helpers, ngap::PLMNIdentity};
use oxirush_utils::identity::{Guti5g, Plmn};
use oxirush_utils::tree;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plmn: Plmn = "310-410".parse()?;
    let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;

    // The 5G-GUTI of a REGISTRATION ACCEPT of oxirush-nas is this value
    let result = NasFGsRegistrationResult::from_result(RegistrationResult::ThreeGppAccess);
    let identity = NasFGsMobileIdentity::from_guti(guti);
    let accept = NasRegistrationAccept::new(result).with_fg_guti(identity);
    let accept = Nas5gsMessage::new_5gmm(accept);
    println!("{}", hex::encode(accept.encode()?));

    // The PLMN identity of a GUAMI of oxirush-ngap has its AP octets
    let guami = helpers::guami(PLMNIdentity(plmn.to_ap_octets()), 0xca, 0x3fe, 0x01)?;
    println!("{:?}: {:02x?}", guami.plmn_identity, guami.plmn_identity.0);

    // The tree of the NAS message is a tree of this crate
    let tree = accept.to_tree();
    println!("{:?}", tree::select(&tree, "/5g-guti/guti/tmsi")?);
    Ok(())
}
```

```text
7e0042010177000bf2130014caff81deadbeef
PLMNIdentity(310-410): [13, 40, 01]
[Number(3735928559)]
```

## IMEI, IMEISV, and the identities of EPS

```rust
use oxirush_utils::identity::{GutiEps, Imei, Imeisv, Imsi, Plmn, Tai5gs, TaiEps};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    // The identity digits of an IMSI in a mobile identity, after its type of identity
    let imsi = Imsi::new("208930000000001")?;
    println!("{:02x?}", imsi.to_nas_octets(1)?);

    // An IMEI as a mobile sends it, and with its check digit; the SVN of an IMEISV
    println!("{}", Imei::with_spare_digit("49015420323751")?);
    println!("{}", Imei::with_check_digit("49015420323751")?);
    println!("{}", Imeisv::new("4901542032375101")?.svn());

    // The GUTI of EPS: its MME group, MME code and M-TMSI; a tracking area of each system
    let plmn = Plmn::new("208", "93")?;
    println!("{}", GutiEps::new(plmn, 0x0002, 0x01, 0xc000_04f4));
    println!("{} {}", Tai5gs::new(plmn, 1)?, TaiEps::new(plmn, 1));
    Ok(())
}
```

```text
[29, 80, 39, 00, 00, 00, 00, 10]
490154203237510
490154203237518
01
4g-guti-20893000201c00004f4
208-93-000001 208-93-0001
```

## The SUCI in the form of a NAI

A SUPI that is no IMSI is a NAI, and so is its SUCI. The grammar of a NAI is
that of RFC 7542; with the feature `idna` its realm is also a name that IDNA
takes as it is written.

```rust
use oxirush_utils::identity::{NaiSuci, nai};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    // The example of TS 23.003 clause 28.7.3: a network specific identifier in clear
    let suci: NaiSuci = "type1.rid678.schid0.useriduser17@example.com".parse()?;
    println!("{:?} {:?} {}", suci.supi_type(), suci.clear(), suci.realm());

    // In a 5GS mobile identity, after the SUPI format of a network specific identifier
    println!("{:#04x}", suci.to_nas_octets()?[0]);

    // A realm has two labels at least
    println!("{}", nai::check_realm("example").unwrap_err());
    Ok(())
}
```

```text
NetworkSpecific Some("user17") example.com
0x11
`realm` is written as a domain name of two labels at least, of letters, digits and hyphens, with no hyphen at either end and 63 octets at most each, as example.com
```

## What other implementations send

`from_nas_octets` gives a value only for the octets that the value is written
as. The readers named `_tolerant` also take what other implementations are
known to send, and say what was not by the clause.
[The documentation](https://docs.rs/oxirush-utils/latest/oxirush_utils/identity/)
lists each tolerance with its clause and its evidence.

```rust
use oxirush_utils::identity::Imeisv;

fn main() -> Result<(), oxirush_utils::identity::Error> {
    // PacketRusher sends an IMEISV with 0000 where its end mark 1111 belongs
    let sent = [0x15, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    println!("{}", Imeisv::from_nas_octets(5, &sent).unwrap_err());

    let (imeisv, faults) = Imeisv::from_nas_octets_tolerant(5, &sent)?;
    println!("{imeisv}, read with: {}", faults[0]);
    Ok(())
}
```

```text
`imeisv` has the half octet 0x0 where its end mark is, and the end mark is 0xf
1110000000000000, read with: `imeisv` has the half octet 0x0 where its end mark is, and the end mark is 0xf
```

## A type as a tree

With the feature `tree`, a message is a `tree::Value`, a JSON value in one
form for every family: octets in lower-case hexadecimal, numbers as JSON
numbers, names read however they are written, members in the order of the
specification. `tree!` writes one as `serde_json::json!` does, in that order.

```rust
use oxirush_utils::{tree, tree::{Members, Reading, Refused, Tree, Value}};

#[derive(Debug)]
struct Slice {
    sst: u8,
    sd: Option<[u8; 3]>,
}

impl Tree for Slice {
    const NAME: &'static str = "S-NSSAI";

    fn to_tree(&self) -> Value {
        let mut members = Members::new();
        members.put("sst", tree::unsigned(self.sst.into()));
        members.put_optional("sd", self.sd.map(|sd| tree::octets(&sd)));
        members.finish()
    }

    fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        let members = reading.members(value, &["sst", "sd"])?;
        Ok(Self {
            sst: reading.member(&members, 0, |value, reading| reading.unsigned(value, 0, 255))? as u8,
            sd: reading.optional(&members, 1, |value, reading| reading.octet_array(value))?,
        })
    }
}

fn main() -> Result<(), oxirush_utils::tree::Error> {
    println!("{}", tree::to_value(&Slice { sst: 1, sd: Some([0, 0, 0xff]) }));

    // A name is read however it is written, and octets in either case
    let read: Slice = tree::from_value(&tree!({"SST": 2, "sd": "0000FF"}))?;
    println!("{read:?}");

    // A refusal says where and why
    println!("{}", tree::from_value::<Slice>(&tree!({"sst": 256})).unwrap_err());
    Ok(())
}
```

```text
{"sst":1,"sd":"0000ff"}
Slice { sst: 2, sd: Some([0, 0, 255]) }
this is not a value of S-NSSAI: a number from 0 to 255 is expected, and 256 is written, at sst
```

## Paths

A path is a JSON pointer: the name of a member, the position of an entry, `*`
for each of them, and `-` for the end of a list in an insertion.

```rust
use oxirush_utils::tree;

fn main() -> Result<(), oxirush_utils::tree::Error> {
    let mut message = tree!({
        "message-type": "registration-request",
        "requested-nssai": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}],
        "t3512-value": null,
    });

    // Select, by names however they are written; what is not there selects nothing
    println!("{:?}", tree::select(&message, "/Requested NSSAI/*/sst")?);
    println!("{:?}", tree::select(&message, "/t3512-value/unit")?);

    // A name that the value does not have is an error
    println!("{}", tree::select(&message, "/requested-nssai/0/sts").unwrap_err());

    // Each value with the path that selects it
    for (path, value) in tree::paths(&message) {
        println!("{path} = {value}");
    }

    // An edit is done whole, or not at all
    tree::set(&mut message, "/requested-nssai/0/sd", tree!("000001"))?;
    tree::insert(&mut message, "/requested-nssai/-", tree!({"sst": 3}))?;
    tree::remove(&mut message, "/requested-nssai/1")?;
    println!("{}", message["requested-nssai"]);
    Ok(())
}
```

```text
[Number(1), Number(2)]
[]
"sts" is not a member of this value, which has sst, at /requested-nssai/0/sts
/message-type = "registration-request"
/requested-nssai/0/sst = 1
/requested-nssai/1/sst = 2
/requested-nssai/1/sd = "0000ff"
[{"sst":1,"sd":"000001"},{"sst":3}]
```

## Any format of serde

With the feature `serde`, a type is serialized and deserialized through its
tree, so YAML and JSON carry the same form.

```rust
use oxirush_utils::tree::{self, Members, Reading, Refused, Tree, Value};

#[derive(Debug, PartialEq)]
struct Timer {
    seconds: u32,
}

impl Tree for Timer {
    const NAME: &'static str = "GPRS timer";

    fn to_tree(&self) -> Value {
        let mut members = Members::new();
        members.put("seconds", tree::unsigned(self.seconds.into()));
        members.finish()
    }

    fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        let members = reading.members(value, &["seconds"])?;
        let seconds = reading.member(&members, 0, |value, reading| reading.unsigned(value, 0, 11160))?;
        Ok(Self { seconds: seconds as u32 })
    }
}

// `Serialize` and `Deserialize`, through the tree
oxirush_utils::impl_serde!(Timer);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timer = Timer { seconds: 3600 };
    println!("{}", serde_json::to_string(&timer)?);
    print!("{}", serde_norway::to_string(&timer)?);
    println!("{:?}", serde_norway::from_str::<Timer>("Seconds: 3600")?);
    Ok(())
}
```

```text
{"seconds":3600}
seconds: 3600
Timer { seconds: 3600 }
```

## Limits

- It knows no codec and no message: the crates of each family do.
- It conceals no SUCI and derives no key:
  [oxirush-security](https://github.com/linouxis9/oxirush-security) does.
- `Supi` checks that a NAI has a realm, and its length, but not its grammar:
  `nai::check_realm` and `nai::check_username` check that.
- A value of `identity` holds what the clauses allow. Octets that are no such
  identity stay octets in the crate that read them.
- A tree is read by serde_json's parser.

## Performance

The tree of the crate against a `serde_json::Value` of the default features,
on the same JSON, and the identities, at 8865ad4, on core 2 of an AMD Ryzen 7
5700G (16 threads, Linux 7.0, Rust 1.98.1), on a machine that ran nothing
else, on 2026-10-18. A time is the lowest of five runs of the benchmark, and
the allocations are those of one call.

| Operation | this crate | serde_json 1 | serde_json / this crate |
|---|---|---|---|
| Read a tree of 925 octets | 2,645 ns, 93 allocations of 6,618 octets | 2,880 ns, 72 allocations of 14,594 octets | about the same |
| Write it | 923 ns ±15% between builds, 5 allocations of 2,845 octets | 2,140 ns, 8 allocations of 2,040 octets | 2.3x |
| Select a value of it | 219 ns, 10 allocations of 385 octets | 238 ns, 8 allocations of 64 octets | about the same |
| Put 100,000 members into an object | 21.0 ms, 113,368 allocations of 17,990,994 octets | 18.6 ms, 115,460 allocations of 10,564,762 octets | 0.89x |
| Read an object of 100,000 members, 1,088,891 octets | 14.6 ms, 109,113 allocations of 21,887,474 octets | 20.7 ms, 115,460 allocations of 10,564,762 octets | 1.4x |
| Read a PLMN from text | 14 ns, 0 allocations of 0 octets | | |
| Read an IMSI from text | 9 ns, 0 allocations of 0 octets | | |
| Read a 5G-GUTI from a 5GS mobile identity | 14 ns, 0 allocations of 0 octets | | |
| Read the SUCI of an IMSI from a 5GS mobile identity | 80 ns, 2 allocations of 20 octets | | |
| Write a PLMN in the order of NGAP | 7 ns, 0 allocations of 0 octets | | |

The layout of the code moves a time by up to 15% from one build of the same
code to another: a ratio within 10%, or within the spread of the runs, is
about the same. A tree is read and written with serde_json, and keeps the
members of an object in their order, where a `serde_json::Value` sorts them.
An object of more than 256 members keeps an index of the hashes of its names,
without a copy of a name: reading n members takes a time that grows as
n log n. A smaller object, as every value of a protocol, is a list that is
searched. The identities allocate nothing but the SUCI, which holds its scheme
output.

```sh
cd benches
cargo bench --bench tree          # the times, with criterion
cargo bench --bench allocations   # the allocations of one call
```

[`benches/src/lib.rs`](https://github.com/linouxis9/oxirush-utils/blob/main/benches/src/lib.rs)
says how to profile one of them.

## Tests and fuzzing

The identities are tested against their clauses, against what Open5GS,
free5GC and PacketRusher sent in lab runs, and against themselves: every
value is read from the octets that it writes, and any octets are a value that
writes them back or none. The tests of oxirush-nas and oxirush-ngap on paths
run here on trees that are written out, without a codec, and so do trees of
65,536 entries, within a time bound. `fuzz/` has two [cargo-fuzz](https://crates.io/crates/cargo-fuzz)
targets: `identity` reads octets and text as each identity, and `paths` takes
any path and any edit on any tree.

```sh
cargo test --all-features
cargo +nightly fuzz run identity -- -max_total_time=60 -max_len=64
cargo +nightly fuzz run paths -- -max_total_time=60 -max_len=512
```

The Rust blocks of this page are checked with what they print
([`tests/readme.rs`](tests/readme.rs) says how). The blocks that use other
crates of the family run in the package of [`family/`](family/), which takes
those crates at the revisions that its manifest names.

A pull request comes with a test for what it changes. Sign off your commits
(`git commit -s`): by contributing you agree to the [Developer Certificate
of Origin](https://developercertificate.org/). [SECURITY.md](SECURITY.md)
says how to report a vulnerability.

## The OxiRush family

- [oxirush-utils](https://github.com/linouxis9/oxirush-utils): the 3GPP identities, and the JSON tree with its paths that the others share.
- [oxirush-aper](https://github.com/linouxis9/oxirush-aper): aligned PER, the runtime of the NGAP and S1AP codecs.
- [oxirush-nas](https://github.com/linouxis9/oxirush-nas): the NAS messages of 5GS and EPS.
- [oxirush-security](https://github.com/linouxis9/oxirush-security): NAS security, AKA and the derivation of keys.
- [oxirush-ngap](https://github.com/linouxis9/oxirush-ngap): NGAP, between a gNB and an AMF.
- [oxirush-s1ap](https://github.com/linouxis9/oxirush-s1ap): S1AP, between an eNB and an MME.
- [oxirush-gtp-u](https://github.com/linouxis9/oxirush-gtp-u): GTP-U tunnels, with a test UPF.
- [oxirush-sctp](https://github.com/linouxis9/oxirush-sctp): SCTP sockets, which carry NGAP and S1AP.

## License

Copyright 2025-2026 Valentin D'Emmanuele. Licensed under the Apache License,
Version 2.0: see [LICENSE](LICENSE).
