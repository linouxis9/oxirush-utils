# Changelog

All notable changes to `oxirush-utils` are recorded here.

## Unreleased

### Added

- `Map::push` adds a member at the end without searching the object for its name,
  for a name that the object does not have, as the members that the tree of a type
  writes one after the other. A debug build panics on a name that is there. Pushing
  4096 members takes 145 µs, where `insert` takes 18 ms; in the tree of a NAS
  REGISTRATION ACCEPT of 23 members, pushing them instead of inserting them takes 3 %
  off the time to build it.
- `tree::read` reads a tree with a function as a value of a type of a name, and refuses it
  as `from_value` does: for a type that a table describes and that has no `Tree`.

### Removed

- `readme/`, the package that compared what each block of the README and of the
  front page of the documentation printed with the text under it: the blocks check
  what they give with `assert_eq!`, and run as documentation tests.

### Fixed

- Fix quadratic time in paths that select many entries of one list. Results, errors
  and the state of the tree after a refused edit are unchanged.
  - `Next::absent_once` no longer scans every place found so far on each call. It
    keeps an index of the valueless places, one per shape. Selecting a member that
    half of the 65,535 entries of a list lack went from 315 ms to 1.5 ms.
  - `remove` and `insert` (and `set` with `null`) remove or insert a run of entries
    of the same list in a single pass, instead of one `Vec::remove` or
    `Vec::insert` per entry. Copying a 65,535-entry list and removing every other
    entry went from 200 ms to 11 ms; copying it and inserting an entry before each
    one went from 1.7 s to 15 ms.
- `set` through `*` gives a member to every entry that lacks it. In the walk of an
  edit, `Next::absent_once` added one valueless place for all the objects that lack
  the member, so `set` gave it to the first of them only: with 7 entries of which 3
  have `sd`, `set` at `/slices/*/sd` changed 4. It now adds a place at each call in
  an edit, as `Next::absent` does; in a walk that selects (`select_in`, `places_in`)
  it still adds a valueless place once per shape. `remove` and `insert` are
  unchanged: they act only where the member is there.

## 0.2.2 - 2026-11-09

### Added

- `Tai5gs::from_tac_octets`, the tracking area identity of a PLMN and the three
  octets of a tracking area code, which cannot fail (3GPP TS 38.413 clause 9.3.3.10).
- `Family::names`, the names that the type of a value has, where the family knows
  them: a path refuses a name that the value does not have with them, as
  `Reading::members` refuses it, without an `Absent::Refused` of the family's own.
- `Next::index`, the position of a segment in the path as it is written: a family
  that refuses a segment can say which path the tree has without it, as the paths
  of a NAS message and of an NGAP or S1AP PDU say for `value` after the name of an
  information element.

### Changed

- A name that a value does not have is refused in one wording by the paths and by
  `Reading::members`: `"sts" is not a member that its type has: it has sst, sd`, or
  `of this value` and the members that it has where the family does not know its
  type. Of more than twelve names, both say the nearest to it, or the first eight
  and how many more, as `Reading::named` does: `Reading::members` listed every name,
  and the paths the first twelve and `…`.

## 0.2.1 - 2026-11-08

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
