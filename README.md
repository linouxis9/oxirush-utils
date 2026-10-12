# oxirush-utils

[![Crates.io](https://img.shields.io/crates/v/oxirush-utils.svg)](https://crates.io/crates/oxirush-utils)
[![Documentation](https://docs.rs/oxirush-utils/badge.svg)](https://docs.rs/oxirush-utils)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

What the OxiRush crates share: the identities of 3GPP networks as values.
`no_std`, without a dependency.

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
writes them back or none.

```sh
cargo test --all-features
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
