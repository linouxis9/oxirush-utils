# Changelog

All notable changes to `oxirush-utils` are recorded here.

## Unreleased (0.1.0)

The first version: what the OxiRush crates share, so that each rule exists
once.

- `identity`: the PLMN with its two codings by name, `to_nas_octets` (3GPP
  TS 24.008 clause 10.5.1.3) and `to_ap_octets` (TS 38.413 clause 9.3.3.5 and
  TS 36.413 clause 9.2.3.8); TBCD strings and the identity digits of a mobile
  identity; `Imsi`, `Imei` with its spare digit or its check digit, `Imeisv`;
  `Guti5g` and `STmsi5g`, `GutiEps` and `STmsiEps`; `Tai5gs` and `TaiEps`; the
  types of identity of TS 24.501, TS 24.301 and TS 24.008. A value is checked
  when it is made, prints whole, and is `Copy`, `Eq`, `Ord` and `Hash`. An
  error names the member at fault and says why.
- A reader of octets is canonical: it gives a value only for the octets that
  the value is written as. The readers named `_tolerant` also take what other
  implementations are known to send, with a list of what was not by the
  clause, which allocates nothing: another end mark than `1111` after an even
  number of identity digits, and the spare bit of the first octet of a
  5G-GUTI or a 5G-S-TMSI.
- `tree`, with the feature `tree`: the tree of a message as a
  `serde_json::Value`. `Tree`, `Members` and `Reading` write a type as a tree
  and read it from one, with refusals that say the member and why. Octets are
  hexadecimal in lower case, read in either; a number is a JSON number; a
  name is read by its letters and its digits, however it is written. The
  members of a tree keep the order in which they were written: the feature
  turns on `serde_json/preserve_order`, for the whole build.
- Paths on a tree: `select`, `paths`, `set`, `remove` and `insert`, with
  4096 octets, 64 segments and 4096 selected values at most, one error type
  with a kind, and edits that are done whole or not at all. `Family` is what
  a family of messages adds where its trees are more than objects and lists;
  the functions that end with `_in` take one. `written_as` brings a value
  that someone wrote to the form that a tree shows at a place, for a
  comparison.
- `tree::serde`, with the feature `serde`: `Serialize` and `Deserialize` for
  a type through its tree, by `impl_serde!` or the two functions that it
  calls.
- `no_std` with `alloc`, no `unsafe`, no dependency without a feature. The
  minimum supported Rust version is 1.88.
