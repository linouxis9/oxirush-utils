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
oxirush-utils = "0.1"
# With the tree and its paths:
# oxirush-utils = { version = "0.1", features = ["tree"] }
```

The minimum supported Rust version is 1.88.

## Identities

A value is checked when it is made, prints whole, and is `Copy`, `Eq`, `Ord`
and `Hash`.

### A PLMN in both orders

An MNC has two or three digits, and how many is part of the value: `208-93`
is not `208-093`. Three octets hold a PLMN in two orders, which differ for an
MNC of three digits.

```rust
use oxirush_utils::identity::Plmn;

fn main() -> Result<(), oxirush_utils::identity::Error> {
    let plmn = Plmn::new("310", "410")?;

    // TS 24.008 clause 10.5.1.3, which NAS uses
    assert_eq!(plmn.to_nas_octets(), [0x13, 0x00, 0x14]);

    // TS 38.413 clause 9.3.3.5 and TS 36.413 clause 9.2.3.8
    assert_eq!(plmn.to_ap_octets(), [0x13, 0x40, 0x01]);
    assert_eq!(Plmn::from_ap_octets([0x13, 0x40, 0x01])?, plmn);

    // What is no PLMN says which member is at fault, and why
    let error = Plmn::new("310", "4").unwrap_err();
    assert_eq!(error.to_string(), "`mnc` has 1 digit, and has 2 or 3");
    Ok(())
}
```

### IMSI, IMEI and IMEISV

```rust
use oxirush_utils::identity::{Imei, Imeisv, Imsi};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    let imsi = Imsi::new("208930000000001")?;
    assert_eq!(imsi.plmn(2)?.to_string(), "208-93");
    assert_eq!(imsi.msin(2)?, "0000000001");

    // The identity digits of a mobile identity, with the type of identity of an IMSI
    assert_eq!(imsi.to_nas_octets(1)?, [0x29, 0x80, 0x39, 0x00, 0x00, 0x00, 0x00, 0x10]);

    // An IMEI as a mobile sends it, and with its check digit
    assert_eq!(Imei::with_spare_digit("49015420323751")?.as_str(), "490154203237510");
    assert_eq!(Imei::with_check_digit("49015420323751")?.as_str(), "490154203237518");
    assert_eq!(Imeisv::new("4901542032375101")?.svn(), "01");
    Ok(())
}
```

### GUTI, S-TMSI and TAI of both systems

```rust
use oxirush_utils::identity::{Guti5g, GutiEps, Plmn, Tai5gs};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    let plmn = Plmn::new("208", "93")?;

    // What Open5GS sent in a REGISTRATION ACCEPT
    let octets = [0xf2, 0x02, 0xf8, 0x39, 0x02, 0x00, 0x40, 0xc0, 0x00, 0x06, 0xcf];
    let guti = Guti5g::from_nas_octets(&octets)?;
    assert_eq!((guti.amf_region_id(), guti.amf_set_id(), guti.tmsi()), (2, 1, 0xc000_06cf));
    assert_eq!(guti.to_string(), "5g-guti-20893020040c00006cf");
    assert_eq!(guti.s_tmsi().to_string(), "5g-s-tmsi-0040c00006cf");

    let eps = GutiEps::new(plmn, 0x0002, 0x01, 0xc000_04f4);
    assert_eq!(eps.to_string().parse::<GutiEps>()?, eps);
    assert_eq!(Tai5gs::new(plmn, 1)?.to_string(), "208-93-000001");
    Ok(())
}
```

### What other implementations send

`from_nas_octets` gives a value only for the octets that the value is written
as. The readers named `_tolerant` also take what other implementations are
known to send, and say what was not by the clause.

```rust
use oxirush_utils::identity::{Fault, Imeisv};

fn main() -> Result<(), oxirush_utils::identity::Error> {
    // PacketRusher sends an IMEISV with 0000 where its end mark 1111 belongs
    let sent = [0x15, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(Imeisv::from_nas_octets(5, &sent).is_err());

    let (imeisv, faults) = Imeisv::from_nas_octets_tolerant(5, &sent)?;
    assert_eq!(imeisv.as_str(), "1110000000000000");
    assert_eq!(faults[0].fault, Fault::EndMark { given: 0 });
    Ok(())
}
```

[The documentation](https://docs.rs/oxirush-utils/latest/oxirush_utils/identity/)
lists each tolerance with its clause and its evidence.

## Trees

With the feature `tree`, a message is a `serde_json::Value` in one form for
every family: octets in lower-case hexadecimal, numbers as JSON numbers, names
read however they are written, members in the order of the specification.

### A type as a tree

```rust
use oxirush_utils::tree::{self, Members, Reading, Refused, Tree};
use serde_json::{Value, json};

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
    let slice = Slice { sst: 1, sd: Some([0, 0, 0xff]) };
    assert_eq!(tree::to_value(&slice), json!({"sst": 1, "sd": "0000ff"}));

    // A name is read however it is written, and octets in either case
    let read: Slice = tree::from_value(&json!({"SST": 2, "sd": "0000FF"}))?;
    assert_eq!((read.sst, read.sd), (2, Some([0, 0, 0xff])));

    // A refusal says where and why
    let error = tree::from_value::<Slice>(&json!({"sst": 256})).unwrap_err();
    assert_eq!(
        error.to_string(),
        "this is not a value of S-NSSAI: a number from 0 to 255 is expected, and 256 is written, at sst"
    );
    Ok(())
}
```

### Paths

A path is a JSON pointer: the name of a member, the position of an entry, `*`
for each of them, and `-` for the end of a list in an insertion.

```rust
use oxirush_utils::tree;
use serde_json::json;

fn main() -> Result<(), oxirush_utils::tree::Error> {
    let mut message = json!({
        "message-type": "registration-request",
        "requested-nssai": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}],
        "t3512-value": null,
    });

    // Select, by names however they are written
    assert_eq!(tree::select(&message, "/Requested NSSAI/*/sst")?, [&json!(1), &json!(2)]);

    // What is not there selects nothing; a name that the value does not have is an error
    assert!(tree::select(&message, "/t3512-value/unit")?.is_empty());
    assert!(tree::select(&message, "/requested-nssai/0/sts").is_err());

    // Each value with the path that selects it
    let paths: Vec<String> = tree::paths(&message).into_iter().map(|(path, _)| path).collect();
    assert_eq!(paths, [
        "/message-type",
        "/requested-nssai/0/sst",
        "/requested-nssai/1/sst",
        "/requested-nssai/1/sd",
    ]);

    // An edit is done whole, or not at all
    tree::set(&mut message, "/requested-nssai/0/sd", json!("000001"))?;
    tree::insert(&mut message, "/requested-nssai/-", json!({"sst": 3}))?;
    tree::remove(&mut message, "/requested-nssai/1")?;
    assert_eq!(message["requested-nssai"], json!([{"sst": 1, "sd": "000001"}, {"sst": 3}]));
    Ok(())
}
```

### The rules of a family

A family of messages whose trees are more than objects and lists implements
`Family`: its root, an entry of a list that a name selects, a member that is
absent and could be there. The functions that end with `_in` take one.

```rust
use oxirush_utils::tree::{self, Family, Next, Place, Root, Step};
use serde_json::{Value, json};

/// A list of `{"id", "value"}` whose entries are selected by their `id`.
struct ById;

impl Family for ById {
    type Shape = ();

    fn root(&self) -> Root<'_> {
        Root { name: "ies", stands_for: &[], required: true }
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
    let mut ies = json!([{"id": "cause", "value": 3}, {"id": "timer", "value": 60}]);
    assert_eq!(tree::select_in(&ById, &ies, "/ies/timer/value")?, [&json!(60)]);
    tree::set_in(&ById, &mut ies, "/ies/cause/value", json!(7))?;
    assert_eq!(ies[0]["value"], 7);
    Ok(())
}
```

### Compare with a value that someone wrote

A tree shows octets in lower case and reads them in either. `written_as`
brings a written value to the form that the tree shows at a place, so that a
comparison does not depend on how its writer spelled it.

```rust
use oxirush_utils::tree::{Form, written_as};
use serde_json::json;

// At a place of octets, C0 is the octets c0
assert_eq!(written_as(Form::Octets, &json!("C0")), json!("c0"));

// At a place of text, C0 is other text than c0
assert_eq!(written_as(Form::Plain, &json!("C0")), json!("C0"));
```

`select_forms_in` gives each selected value with the form that its family
says it has.

### Any format of serde

With the feature `serde`, a type is serialized and deserialized through its
tree, so YAML and JSON carry the same form.

```rust
use oxirush_utils::tree::{self, Members, Reading, Refused, Tree};
use serde_json::Value;

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

fn main() {
    let timer = Timer { seconds: 3600 };
    assert_eq!(serde_json::to_string(&timer).unwrap(), r#"{"seconds":3600}"#);
    assert_eq!(serde_json::from_str::<Timer>(r#"{"Seconds": 3600}"#).unwrap(), timer);
}
```

## Features

| Feature | Adds | Through |
|---------|------|---------|
| `tree` | The tree of a message, its forms and its paths | `serde_json` with `preserve_order` |
| `serde` | `Serialize` and `Deserialize` for a type through its tree | `serde_core` |

No feature is on by default.

The feature `tree` turns on `serde_json/preserve_order`, so that the members
of a tree keep the order of the specification whatever else a build enables.
Cargo unifies features: in a program that depends on this crate with `tree`,
every `serde_json::Map` keeps the order in which its members were inserted,
those of the program too.

## What it does not do

- It knows no codec and no message: the crates of each family do.
- It conceals no SUCI and derives no key:
  [oxirush-security](https://github.com/linouxis9/oxirush-security) does.
- A value of `identity` holds what the clauses allow. Octets that are no such
  identity stay octets in the crate that read them.

## Tests and fuzzing

The identities are tested against their clauses, against what Open5GS,
free5GC and PacketRusher sent in lab runs, and against themselves: every
value is read from the octets that it writes, and any octets are a value that
writes them back or none. The paths hold the tests of the crates of messages
on trees that are written out, and a tree of 65,535 entries within a time
bound.

```sh
cargo test --all-features
```

`fuzz/` has two [cargo-fuzz](https://crates.io/crates/cargo-fuzz) targets:
`identity` reads octets and text as each identity, and `paths` takes any path
and any edit on any tree.

```sh
cargo +nightly fuzz run identity -- -max_total_time=60 -max_len=64
cargo +nightly fuzz run paths -- -max_total_time=60 -max_len=512
```

## OxiRush crates

[oxirush-nas](https://github.com/linouxis9/oxirush-nas) (NAS) ·
[oxirush-ngap](https://github.com/linouxis9/oxirush-ngap) and
[oxirush-s1ap](https://github.com/linouxis9/oxirush-s1ap) (NGAP and S1AP) ·
[oxirush-aper](https://github.com/linouxis9/oxirush-aper) (aligned PER) ·
[oxirush-security](https://github.com/linouxis9/oxirush-security) (NAS keys
and algorithms) ·
[oxirush-gtp-u](https://github.com/linouxis9/oxirush-gtp-u) (user plane) ·
[oxirush-sctp](https://github.com/linouxis9/oxirush-sctp) (transport)

## Contributing

Open a pull request against `main` with a test for what it changes.
`cargo test --all-features` and `cargo clippy --all-targets --all-features`
pass on every commit. Sign off your commits (`git commit -s`), which agrees
to the [Developer Certificate of Origin](https://developercertificate.org/).
[SECURITY.md](SECURITY.md) says how to report a vulnerability.

## License

Apache-2.0. See [LICENSE](LICENSE).
