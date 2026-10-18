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

This page describes 0.1.0, which is not on crates.io yet.

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

No feature is on by default. Of the features of `serde_json`, `tree` turns on
`float_roundtrip` alone: the JSON of a program keeps the order that the
program chose, and a number with a fraction is read as the nearest f64.

Until 0.1.0 is on crates.io, a program takes the crate from its repository:

```toml
[dependencies]
oxirush-utils = { git = "https://github.com/linouxis9/oxirush-utils", features = ["tree"] }
```

The minimum supported Rust version is 1.88.

## A PLMN in both orders

A value of `identity` is checked when it is made, and prints whole. An MNC has
two or three digits, and how many is part of the value: `208-93` is not
`208-093`.

```rust
use oxirush_utils::identity::Plmn;

fn main() -> Result<(), oxirush_utils::identity::Error> {
    let plmn = Plmn::new("310", "410")?;

    // TS 24.008 clause 10.5.1.3 for NAS, TS 38.413 clause 9.3.3.5 for NGAP and S1AP
    println!("{:02x?} {:02x?}", plmn.to_nas_octets(), plmn.to_ap_octets());
    println!("{}", Plmn::from_ap_octets([0x02, 0xf8, 0x39])?);

    // What is no PLMN says which member is at fault, and why
    println!("{}", Plmn::new("310", "4").unwrap_err());
    Ok(())
}
```

```text
[13, 00, 14] [13, 40, 01]
208-93
`mnc` has 1 digit, and has 2 or 3
```

## IMSI, IMEI and IMEISV

```rust
use oxirush_utils::identity::{Imei, Imeisv, Imsi};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    let imsi = Imsi::new("208930000000001")?;
    println!("{} {}", imsi.plmn(2)?, imsi.msin(2)?);

    // The identity digits of a mobile identity, with the type of identity of an IMSI
    println!("{:02x?}", imsi.to_nas_octets(1)?);

    // An IMEI as a mobile sends it, and with its check digit
    println!("{}", Imei::with_spare_digit("49015420323751")?);
    println!("{}", Imei::with_check_digit("49015420323751")?);
    println!("{}", Imeisv::new("4901542032375101")?.svn());
    Ok(())
}
```

```text
208-93 0000000001
[29, 80, 39, 00, 00, 00, 00, 10]
490154203237510
490154203237518
01
```

## GUTI, S-TMSI and TAI of both systems

```rust
use oxirush_utils::identity::{Guti5g, GutiEps, Plmn, Tai5gs};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    // What Open5GS sent in a REGISTRATION ACCEPT
    let octets = [0xf2, 0x02, 0xf8, 0x39, 0x02, 0x00, 0x40, 0xc0, 0x00, 0x06, 0xcf];
    let guti = Guti5g::from_nas_octets(&octets)?;
    println!("{guti} {}", guti.s_tmsi());
    println!("{} {} {:#x}", guti.amf_region_id(), guti.amf_set_id(), guti.tmsi());

    let plmn = Plmn::new("208", "93")?;
    let eps = GutiEps::new(plmn, 0x0002, 0x01, 0xc000_04f4);
    println!("{eps} {}", Tai5gs::new(plmn, 1)?);
    Ok(())
}
```

```text
5g-guti-20893020040c00006cf 5g-s-tmsi-0040c00006cf
2 1 0xc00006cf
4g-guti-20893000201c00004f4 208-93-000001
```

## SUPI, and the SUCI of an IMSI

A SUCI is the value of its parts, in its two forms. Concealing an IMSI and
reading a concealed one need keys, and are no part of this crate.

```rust
use oxirush_utils::identity::{ImsiSuci, Plmn, RoutingIndicator, Supi, SupiType};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    // The example of TS 23.003 clause 2.2B
    let plmn = Plmn::new("234", "15")?;
    let suci = ImsiSuci::null(plmn, RoutingIndicator::new("678")?, "0999999999")?;
    println!("{suci}: {:02x?}", suci.to_nas_octets());

    // What PacketRusher sent in a REGISTRATION REQUEST: under the null scheme, the SUPI is in it
    let sent = [0x01, 0x02, 0xf8, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10];
    let suci = ImsiSuci::from_nas_octets(&sent)?;
    println!("{suci} {}", Supi::from(suci.imsi().unwrap()));

    // A GCI is 3 in TS 23.003 clause 2.2B and 2 in TS 24.501 table 9.11.3.4.1
    println!("{} {}", SupiType::Gci.type_number(), SupiType::Gci.nas_format());
    Ok(())
}
```

```text
suci-0-234-15-678-0-0-0999999999: [01, 32, f4, 51, 76, f8, 00, 00, 90, 99, 99, 99, 99]
suci-0-208-93-0000-0-0-0000000001 imsi-208930000000001
3 2
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

## The rules of a family

A family of messages whose trees are more than objects and lists implements
`Family`: its root, an entry of a list that a name selects, a member that is
absent and could be there. The functions that end with `_in` take one.

```rust
use oxirush_utils::{tree, tree::{Family, Next, Place, Root, Step, Value}};

/// A list of `{"id", "value"}` whose entries are selected by their `id`.
struct ById;

impl Family for ById {
    type Shape = ();

    fn root(&self) -> Root<'_> {
        Root::new("ies", &[], true)
    }

    fn step<'v>(
        &self,
        place: &Place<'v, ()>,
        segment: &str,
        next: &mut Next<'v, ()>,
    ) -> Result<Step<()>, tree::Error> {
        let Some(entries) = place.value().and_then(Value::as_array) else {
            return Ok(Step::Plain);
        };
        let named = |entry: &Value| entry["id"].as_str() == Some(segment);
        if !entries.iter().any(named) {
            return Ok(Step::Plain);
        }
        for (position, entry) in entries.iter().enumerate().filter(|(_, entry)| named(entry)) {
            next.entry(position, entry, ());
        }
        Ok(Step::Taken)
    }
}

fn main() -> Result<(), tree::Error> {
    let mut ies = tree!([{"id": "cause", "value": 3}, {"id": "timer", "value": 60}]);
    println!("{:?}", tree::select_in(&ById, &ies, "/ies/timer/value")?);
    tree::set_in(&ById, &mut ies, "/ies/cause/value", tree!(7))?;
    println!("{ies}");
    Ok(())
}
```

```text
[Number(60)]
[{"id":"cause","value":7},{"id":"timer","value":60}]
```

## Compare with a value that someone wrote

A tree shows octets in lower case and reads them in either. `written_as`
brings a written value to the form that the tree shows at a place, so that a
comparison does not depend on how its writer spelled it. `select_forms_in`
gives each selected value with the form that its family says it has.

```rust
use oxirush_utils::{tree, tree::{Form, written_as}};

// At a place of octets C0 is the octets c0, and at a place of text it stays C0
println!("{}", written_as(Form::Octets, &tree!("C0")));
println!("{}", written_as(Form::Plain, &tree!("C0")));
```

```text
"c0"
"C0"
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
    print!("{}", serde_yaml::to_string(&timer)?);
    println!("{:?}", serde_yaml::from_str::<Timer>("Seconds: 3600")?);
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
- `Supi` checks of a NAI that it has a realm and its length, not its
  grammar, which `nai::check_realm` and `nai::check_username` check.
- A value of `identity` holds what the clauses allow. Octets that are no such
  identity stay octets in the crate that read them.
- A tree is no faster a JSON reader than `serde_json`, on which it is built.

## Performance

The tree of the crate against a `serde_json::Value` of the default features,
on the same JSON, and the identities, on one core of an AMD Ryzen 7 5700G (16
threads, Linux 7.0, Rust 1.98.1), measured on a quiet machine on 2026-10-18.
A time is the fastest of three runs of the call, and the median of three runs
of the program.

| Operation | this crate | serde_json 1 | serde_json takes |
|---|---|---|---|
| Read a tree of 925 octets | 2,659 ns, 93 allocations of 6,618 octets | 2,677 ns, 72 allocations of 14,594 octets | 1.0x |
| Write it | 773 ns, 5 allocations of 2,845 octets | 2,143 ns, 8 allocations of 2,040 octets | 2.8x |
| Select a value of it | 229 ns, 10 allocations of 385 octets | 247 ns, 8 allocations of 64 octets | 1.1x |
| Put 100,000 members into an object | 42.8 ms, 215,477 allocations of 21,752,076 octets | 18.5 ms, 115,460 allocations of 10,564,762 octets | 0.4x |
| Read an object of 100,000 members, 1,088,891 octets | 45.5 ms, 215,478 allocations of 23,352,076 octets | 21.5 ms, 115,460 allocations of 10,564,762 octets | 0.5x |
| Read a PLMN from text | 13 ns, 0 allocations of 0 octets | | |
| Read an IMSI from text | 9 ns, 0 allocations of 0 octets | | |
| Read a 5G-GUTI from a 5GS mobile identity | 14 ns, 0 allocations of 0 octets | | |
| Read the SUCI of an IMSI from a 5GS mobile identity | 83 ns, 2 allocations of 20 octets | | |
| Write a PLMN in the order of NGAP | 7 ns, 0 allocations of 0 octets | | |

A tree is read and written with serde_json, and keeps its members in their
order where a `serde_json::Value` sorts them. An object of more than 32 members
keeps an index of its names, a copy of each, so that an object of n members is
read in a time that grows as n log n: twice the time and the allocations of
serde_json for 100,000 members. The identities allocate nothing but the SUCI,
which holds its scheme output.

```sh
cargo run --release --manifest-path benches/Cargo.toml
```

## Tests and fuzzing

The identities are tested against their clauses, against what Open5GS,
free5GC and PacketRusher sent in lab runs, and against themselves: every
value is read from the octets that it writes, and any octets are a value that
writes them back or none. The paths hold the tests of the crates of messages
on trees that are written out, and trees of 65,536 entries within a time
bound. `fuzz/` has two [cargo-fuzz](https://crates.io/crates/cargo-fuzz)
targets: `identity` reads octets and text as each identity, and `paths` takes
any path and any edit on any tree.

```sh
cargo test --all-features
cargo +nightly fuzz run identity -- -max_total_time=60 -max_len=64
cargo +nightly fuzz run paths -- -max_total_time=60 -max_len=512
```

A pull request comes with a test for what it changes; the Rust blocks of
this page are checked with what they print ([`tests/readme.rs`](tests/readme.rs)
says how). Sign off your commits (`git commit -s`): by contributing you agree
to the [Developer Certificate of Origin](https://developercertificate.org/).
[SECURITY.md](SECURITY.md) says how to report a vulnerability.

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
