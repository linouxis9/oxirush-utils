# Changelog

All notable changes to `oxirush-utils` are recorded here.

## 0.2.1 - unreleased

### Added

- `Next::entry_member`, for a family whose list holds each value under a name, as
  the information elements of a message, `[{"Cause": …}]`: one segment selects the
  value that an entry holds under that name, and an edit there edits the value and
  keeps the entry and its other members. `Next::beside` goes from that value to
  another member of its entry, as the criticality beside an information element.

### Changed

- `set` adds an entry at the position of the length of a list, and makes a list of a
  place that is `null` for the entry at position 0; below an entry that it adds, it
  takes a member as of an empty object, or the entry at position 0. A position past the
  length still selects nothing.

## 0.2.0 - 2026-11-07

### Added

- `identity::Suci`, the SUCI of either form (`Imsi(ImsiSuci)` or `Nai(NaiSuci)`),
  read from a 5GS mobile identity by its SUPI format, with its parts, the NAI form
  of the SUCI of an IMSI, and the text of the service-based interfaces.
- `identity::Algorithm`, the identity of a security algorithm, which ciphering and
  integrity share in 5GS and EPS.
- `Guti5g::to_mapped_eps` and `GutiEps::to_mapped_5g` (3GPP TS 23.003 clause 2.10.2).

### Changed

- The feature `serde` is part of `tree`.
- `Supi` owns its text and needs the feature `alloc`.
- `Reading::member` and `Reading::optional` take the name of the member, not its
  position; a name that the type does not list panics.
- `Value::get` takes the name of a member.
- `to_nas_octets` of `Imsi`, `Imei` and `Imeisv` cannot fail: it takes the three low
  bits of the type of identity.
- An object of a tree is a list without an index; one that is read has 4096 members
  at most.

### Removed

- `Value::pointer`, `pointer_mut`, `take`, `as_null`, `is_u64`, `is_i64`, `is_f64`,
  the trait `tree::Index`, and the comparisons of a tree with a `serde_json::Value`,
  a number or a string.
- `Map::remove` and `Map::shift_insert`, `Entry::or_insert_with`,
  `Entry::or_default`.
- `tree::to_value`, `tree::written_as`, `tree::select_forms_in`, `tree::Form` and
  `Family::form`, and the constants `MAX_OCTETS`, `MAX_SEGMENTS`, `MAX_SELECTED`.
- `bits()` of the types of identity, `ImsiSuci::of`, `Guti5g::of`,
  `STmsiEps::to_u64`, `Supi::into_owned`.

### Migration from 0.1

| 0.1 | 0.2 |
|-----|-----|
| `features = ["serde"]` | `features = ["tree"]` |
| `Supi<'a>`, `supi.into_owned()` | `Supi`, `supi` |
| `reading.member(&members, 0, ..)` | `reading.member(&members, "sst", ..)` |
| `tree::to_value(&x)` | `x.to_tree()` |
| `value.pointer("/a/0")` | `tree::select(&value, "/a/0")`, or `value["a"][0]` |
| `value.get(0)` | `value[0]`, or `value.as_array()?.get(0)` |
| `value.take()` | `core::mem::take(&mut value)` |
| `assert_eq!(value, 1)`, `value == json` | `assert_eq!(value, tree!(1))`, `value == Value::from(json)` |
| `map.remove(name)` | `map.shift_remove(name)` |
| `entry.or_default()` | `entry.or_insert(Value::Null)` |
| `kind.bits()` | `u8::from(kind)` |
| `imsi.to_nas_octets(1)?` | `imsi.to_nas_octets(1)` |
| `ImsiSuci::of(imsi, 2)` | `ImsiSuci::null(imsi.plmn(2)?, RoutingIndicator::default(), imsi.msin(2)?)` |
| `Guti5g::of(plmn, region, s_tmsi)` | `Guti5g::new(plmn, region, s_tmsi.amf_set_id(), s_tmsi.amf_pointer(), s_tmsi.tmsi())?` |

## 0.1.0 - 2026-10-24

The first version.

- `identity`: the PLMN with its NAS and AP codings, TBCD strings, `Imsi`, `Imei`,
  `Imeisv`, `Supi` and `SupiType`, `ImsiSuci` and `NaiSuci` with `RoutingIndicator`
  and `ProtectionScheme`, `Guti5g`, `STmsi5g`, `GutiEps`, `STmsiEps`, `Tai5gs`,
  `TaiEps`, the types of identity, and the grammar of a NAI (`nai::check_realm`,
  `nai::check_username`, with the feature `idna` for internationalized names).
  Values are checked when they are made and print whole; readers of octets are
  canonical, and those named `_tolerant` take what other implementations send.
- `tree`, with the feature `tree`: a JSON tree that keeps the order of its members,
  `tree!`, `Tree` with `Members` and `Reading`, paths with `select`, `paths`, `set`,
  `remove` and `insert`, and `Family` for the rules of a family of messages.
- `tree::serde`, with the feature `serde`: `Serialize` and `Deserialize` through the
  tree, by `impl_serde!`.
- `no_std`, no `unsafe`, no dependency without a feature; MSRV 1.88.
