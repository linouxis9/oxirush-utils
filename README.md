# oxirush-utils

[![Crates.io](https://img.shields.io/crates/v/oxirush-utils.svg)](https://crates.io/crates/oxirush-utils)
[![Documentation](https://docs.rs/oxirush-utils/badge.svg)](https://docs.rs/oxirush-utils)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

What the OxiRush crates share, in Rust: the identities of 3GPP networks as
values, and one JSON tree with paths for the messages of every protocol, in
`no_std` and with no dependency unless a feature asks for one.

```rust
use oxirush_utils::identity::{Guti5g, Plmn};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plmn: Plmn = "310-410".parse()?;
    let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;

    // The three octets of a PLMN in a NAS message, and in an NGAP or S1AP message
    assert_eq!(plmn.to_string(), "310-410");
    assert_eq!(plmn.to_nas_octets(), [0x13, 0x00, 0x14]);
    assert_eq!(plmn.to_ap_octets(), [0x13, 0x40, 0x01]);

    // A 5G-GUTI as text, and as a 5GS mobile identity
    assert_eq!(guti.to_string(), "5g-guti-310410caff81deadbeef");
    assert_eq!(guti.to_nas_octets(), [0xf2, 0x13, 0x00, 0x14, 0xca, 0xff, 0x81, 0xde, 0xad, 0xbe, 0xef]);
    Ok(())
}
```

Each example shows what it gives with `assert_eq!`, and runs as a test of the crate.

## Installation

```toml
[dependencies]
oxirush-utils = { version = "0.2", features = ["alloc"] }
# With the tree and its paths, which need an allocator too:
# oxirush-utils = { version = "0.2", features = ["tree"] }
# Without an allocator, the values that have their digits in place:
# oxirush-utils = "0.2"
```

| Feature | Adds | Through |
|---------|------|---------|
| `alloc` | What holds a text or a list: the SUPI, the SUCIs, TBCD strings, identity digits as octets | |
| `tree` | The tree of a message, its forms and its paths, and `Serialize` and `Deserialize` for a type through its tree | `serde_json`, `serde_core` |
| `idna` | The realm and the username of a NAI checked as internationalized names too: Unicode normalization form C, and a realm that IDNA takes as it is written | `idna`, `unicode-normalization` |

No feature is on by default. `tree` turns on the `float_roundtrip` feature of
`serde_json`, so a number with a fraction is read as the nearest f64. A tree
keeps the members of an object in the order the program wrote them.

The minimum supported Rust version is 1.88.

## An identity through its forms

A value of `identity` is checked when it is made, and prints whole, in the
text of the service based interfaces. An MNC has two or three digits, and
how many is part of the value: `208-93` is not `208-093`.

```rust
use oxirush_utils::identity::{Guti5g, Imsi, ImsiSuci, Plmn, RoutingIndicator, Supi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // An IMSI whose MNC has three digits: its PLMN and its MSIN
    let imsi = Imsi::new("310410123456789")?;
    let (plmn, msin) = (imsi.plmn(3)?, imsi.msin(3)?);
    assert_eq!((plmn.to_string(), msin.to_string()), ("310-410".into(), "123456789".into()));

    // Its SUCI under the null scheme, as text and as a 5GS mobile identity,
    // and the SUPI that the SUCI gives
    let suci = ImsiSuci::null(plmn, RoutingIndicator::default(), msin)?;
    assert_eq!(suci.to_string(), "suci-0-310-410-0-0-0-123456789");
    assert_eq!(
        suci.to_nas_octets(),
        [0x01, 0x13, 0x00, 0x14, 0xf0, 0xff, 0x00, 0x00, 0x21, 0x43, 0x65, 0x87, 0xf9],
    );
    let supi = Supi::from(suci.imsi().ok_or("a concealed MSIN")?);
    assert_eq!(supi.to_string(), "imsi-310410123456789");

    // The 5G-GUTI that an AMF assigns: AMF region, set and pointer, and the
    // 5G-TMSI. It is read back from its octets, and has a 5G-S-TMSI
    let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;
    let read = Guti5g::from_nas_octets(&guti.to_nas_octets())?;
    assert_eq!(read.s_tmsi().to_string(), "5g-s-tmsi-ff81deadbeef");

    // A PLMN that is not one says which member is wrong, and why
    let error = Plmn::new("208", "9").unwrap_err();
    assert_eq!(error.to_string(), "`mnc` has 1 digit, and must have 2 or 3");
    Ok(())
}
```

## Where each form goes

The NAS octets of an identity are the value of an IE of
[oxirush-nas](https://github.com/linouxis9/oxirush-nas). The AP octets of a
PLMN are those of the identities of
[oxirush-ngap](https://github.com/linouxis9/oxirush-ngap) and
[oxirush-s1ap](https://github.com/linouxis9/oxirush-s1ap). The messages of
all three are trees of this crate, read by the same paths: their READMEs show
how.

## IMEI, IMEISV, and the identities of EPS

```rust
use oxirush_utils::identity::{GutiEps, Imei, Imeisv, Imsi, Plmn, Tai5gs, TaiEps};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The identity digits of an IMSI in a mobile identity, after its type of identity
    let imsi = Imsi::new("208930000000001")?;
    assert_eq!(imsi.to_nas_octets(1), [0x29, 0x80, 0x39, 0x00, 0x00, 0x00, 0x00, 0x10]);

    // An IMEI as a mobile sends it, and with its check digit; the SVN of an IMEISV
    assert_eq!(Imei::with_spare_digit("49015420323751")?.to_string(), "490154203237510");
    assert_eq!(Imei::with_check_digit("49015420323751")?.to_string(), "490154203237518");
    assert_eq!(Imeisv::new("4901542032375101")?.svn().to_string(), "01");

    // The GUTI of EPS: its MME group, MME code and M-TMSI; a tracking area of each system
    let plmn = Plmn::new("208", "93")?;
    let guti = GutiEps::new(plmn, 0x0002, 0x01, 0xc000_04f4);
    assert_eq!(guti.to_string(), "4g-guti-20893000201c00004f4");
    assert_eq!(Tai5gs::new(plmn, 1)?.to_string(), "208-93-000001");
    assert_eq!(TaiEps::new(plmn, 1).to_string(), "208-93-0001");
    Ok(())
}
```

## The SUCI in the form of a NAI

A SUPI that is not an IMSI is a NAI, and so is its SUCI. A NAI follows the
grammar of RFC 7542; with the `idna` feature, its realm must also be a name
that IDNA takes as written.

```rust
use oxirush_utils::identity::{NaiSuci, nai};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The example of TS 23.003 clause 28.7.3: a network specific identifier in clear
    let suci: NaiSuci = "type1.rid678.schid0.useriduser17@example.com".parse()?;
    assert_eq!(format!("{:?}", suci.supi_type()), "NetworkSpecific");
    assert_eq!((suci.clear(), suci.realm()), (Some("user17"), "example.com"));

    // In a 5GS mobile identity, after the SUPI format of a network specific identifier
    assert_eq!(suci.to_nas_octets()?[0], 0x11);

    // A realm has two labels at least
    assert_eq!(
        nai::check_realm("example").unwrap_err().to_string(),
        "`realm` is written as a domain name of two labels at least, of letters, digits and \
         hyphens, with no hyphen at either end and 63 octets at most each, as example.com",
    );
    Ok(())
}
```

## What other implementations send

`from_nas_octets` takes only the octets that the clause writes. The readers
named `_tolerant` also take what other implementations are known to send,
and say what departs from the clause.
[The documentation](https://docs.rs/oxirush-utils/latest/oxirush_utils/identity/)
lists each tolerance with its clause and its evidence.

```rust
use oxirush_utils::identity::Imeisv;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // PacketRusher sends an IMEISV with 0000 where its end mark 1111 belongs
    let sent = [0x15, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let fault = "`imeisv` has the half octet 0x0 where its end mark is, and the end mark is 0xf";
    assert_eq!(Imeisv::from_nas_octets(5, &sent).unwrap_err().to_string(), fault);

    let (imeisv, faults) = Imeisv::from_nas_octets_tolerant(5, &sent)?;
    assert_eq!(imeisv.to_string(), "1110000000000000");
    assert_eq!(faults[0].to_string(), fault);
    Ok(())
}
```

## A type as a tree

With the `tree` feature, a message is a `tree::Value`: a JSON value in one
form for every protocol, with octets in lower-case hexadecimal, numbers as
JSON numbers, names read however they are written, and members in the order
of the specification. `tree!` writes one as `serde_json::json!` does, and
keeps that order.

```rust
use oxirush_utils::{tree, tree::{Members, Reading, Refused, Tree, Value}};

#[derive(Debug, PartialEq)]
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
            sst: reading.member(&members, "sst", |value, reading| reading.unsigned(value, 0, 255))? as u8,
            sd: reading.optional(&members, "sd", |value, reading| reading.octet_array(value))?,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let slice = Slice { sst: 1, sd: Some([0, 0, 0xff]) };
    assert_eq!(slice.to_tree().to_string(), r#"{"sst":1,"sd":"0000ff"}"#);

    // A name is read however it is written, and octets in either case
    let read: Slice = tree::from_value(&tree!({"SST": 2, "sd": "0000FF"}))?;
    assert_eq!(read, Slice { sst: 2, sd: Some([0, 0, 0xff]) });

    // A refusal says where and why
    assert_eq!(
        tree::from_value::<Slice>(&tree!({"sst": 256})).unwrap_err().to_string(),
        "this is not a value of S-NSSAI: a number from 0 to 255 is expected, and 256 is written, at sst",
    );
    Ok(())
}
```

## Paths

A path is a JSON pointer: the name of a member, the position of an entry, `*`
for each of them, and `-` for the end of a list in an insertion.

```rust
use oxirush_utils::tree;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut message = tree!({
        "message-type": "registration-request",
        "requested-nssai": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}],
        "t3512-value": null,
    });

    // Select, by names however they are written; what is not there selects nothing
    assert_eq!(tree::select(&message, "/Requested NSSAI/*/sst")?, [&tree!(1), &tree!(2)]);
    assert!(tree::select(&message, "/t3512-value/unit")?.is_empty());

    // A name that the value does not have is an error
    assert_eq!(
        tree::select(&message, "/requested-nssai/0/sts").unwrap_err().to_string(),
        r#""sts" is not a member of this value: it has sst, at /requested-nssai/0/sts"#,
    );

    // Each value with the path that selects it
    let paths: Vec<String> =
        tree::paths(&message).into_iter().map(|(path, value)| format!("{path} = {value}")).collect();
    assert_eq!(
        paths,
        [
            r#"/message-type = "registration-request""#,
            "/requested-nssai/0/sst = 1",
            "/requested-nssai/1/sst = 2",
            r#"/requested-nssai/1/sd = "0000ff""#,
        ],
    );

    // An edit is done whole, or not at all
    tree::set(&mut message, "/requested-nssai/0/sd", tree!("000001"))?;
    tree::insert(&mut message, "/requested-nssai/-", tree!({"sst": 3}))?;
    tree::remove(&mut message, "/requested-nssai/1")?;
    assert_eq!(message["requested-nssai"], tree!([{"sst": 1, "sd": "000001"}, {"sst": 3}]));
    Ok(())
}
```

## Any format of serde

A type is serialized and deserialized through its tree, so YAML and JSON
carry the same form.

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
        let seconds = reading.member(&members, "seconds", |value, reading| reading.unsigned(value, 0, 11160))?;
        Ok(Self { seconds: seconds as u32 })
    }
}

// `Serialize` and `Deserialize`, through the tree
oxirush_utils::impl_serde!(Timer);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timer = Timer { seconds: 3600 };
    assert_eq!(serde_json::to_string(&timer)?, r#"{"seconds":3600}"#);
    assert_eq!(serde_norway::to_string(&timer)?, "seconds: 3600\n");
    assert_eq!(serde_norway::from_str::<Timer>("Seconds: 3600")?, timer);
    Ok(())
}
```

## Limits

- It knows no codec and no message: the crates of each protocol do.
- It conceals no SUCI and derives no key:
  [oxirush-security](https://github.com/linouxis9/oxirush-security) does.
- `Supi` checks that a NAI has a realm, and its length, but not its grammar:
  `nai::check_realm` and `nai::check_username` check that.
- A value of `identity` holds what the clauses allow. Octets that are no such
  identity stay octets in the crate that reads them.
- It has no JSON parser of its own: serde_json's reads a tree.

## Performance

The tree of the crate against a `serde_json::Value` of the default
features, on the same JSON, at 8865ad4 on core 2 of an AMD Ryzen 7 5700G
(Linux 7.0, Rust 1.98.1), on 2026-10-18. A ratio is the time of serde_json
over this crate's, and "about the same" is within 10 %:

| Operation | This crate | serde_json 1 | serde_json / this crate |
|---|---|---|---|
| Read a tree of 925 octets | 2,645 ns | 2,880 ns | about the same |
| Write it | 923 ns ±15% between builds | 2,140 ns | 2.3x |
| Select a value of it | 219 ns | 238 ns | about the same |
| Put 4096 members into an object, one after the other | 15.9 ms | 0.65 ms | 0.04x |
| Push 4096 new members, one after the other | 145 µs | | |
| Read an object of 4096 members, 39,851 octets | 380 µs | 774 µs | 2.0x |

A tree keeps the members of an object in their order, where a
`serde_json::Value` sorts them. An object is a list that is searched for a
name, which takes less than a map for the few dozen members of a value of a
protocol: putting n members one after the other takes a time that grows as
n², pushing n members whose names are new (`Map::push`) a time that grows as
n, and an object that is read has 4096 members at most. The two rows of 4096
members that are put and read were measured on 2026-11-07, when an object
became a list, and the row that pushes them on 2026-11-13. Reading a PLMN, an IMSI or a 5G-GUTI takes 9
to 14 ns without an allocation, and the SUCI of an IMSI 80 ns.
[`benches/`](benches/README.md) has the allocations, the other identities and
the method. In `benches/`:

```sh
cargo bench --bench tree
```

## Tests and fuzzing

The identities are tested against their clauses, against what Open5GS,
free5GC and PacketRusher sent in lab runs, and against themselves: every
value is read back from the octets it writes, and any octets are either a
value that writes them back or no value. The tests of oxirush-nas and
oxirush-ngap on paths run here too, on trees written out without a codec,
and so do trees of 65,536 entries, within a time bound. `fuzz/` has three
[cargo-fuzz](https://crates.io/crates/cargo-fuzz) targets: `identity` reads
octets and text as each identity, `paths` takes any path and any edit on any
tree, and `forms` reads any JSON with each reader of a tree and holds the
forms of a tree and the segments of its paths to what they write.

```sh
cargo test --all-features
cargo +nightly fuzz run identity -- -max_total_time=60 -max_len=64
cargo +nightly fuzz run paths -- -max_total_time=60 -max_len=512
```

Each Rust block of this page checks what it gives with `assert_eq!`, and runs
as a documentation test of the crate.

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
