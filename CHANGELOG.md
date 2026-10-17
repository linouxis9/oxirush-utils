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
  when it is made, prints whole, and is `Eq`, `Ord` and `Hash`, and `Copy`
  where it holds no text. An error names the member at fault and says why: a
  character that is no digit is refused as that character, before the digits
  are counted.
- `Supi` with its `SupiType`, whose number in TS 23.003 clause 2.2B and whose
  SUPI format in TS 24.501 table 9.11.3.4.1 differ for a GLI and a GCI.
  `ImsiSuci`, the SUCI of an IMSI as the value of its parts, with
  `RoutingIndicator` and `ProtectionScheme`: the 5GS mobile identity of
  TS 24.501 figure 9.11.3.4.3, and the text `suci-0-208-93-0-0-0-0000000001`.
  The crate conceals nothing and reads no concealed identity.
- `NaiSuci`, the SUCI in the form of a NAI that a network specific
  identifier, a GCI or a GLI has, and an IMSI as a NAI (TS 23.003 clauses
  28.7.3, 28.15.5 and 28.16.5): the text
  `type1.rid678.schid0.useriduser17@example.com` and the 5GS mobile identity
  that holds it, with the rules of each SUPI type, the key identifier of the
  scheme and the fields of a profile of ECIES. The scheme output of either
  form that is made has the lengths of the parts of its profile (TS 33.501
  annex C.3.4): 41 to 45 octets for an IMSI with profile A, 42 to 46 with
  profile B.
  `nai::check_realm` and `nai::check_username` check the grammar of RFC 7542,
  and `nai::home_network_domain` writes the realm of an IMSI. With the
  feature `idna` a realm and a username are also in Unicode normalization
  form C, and a realm is a name that IDNA takes as it is written, in either
  case (TS 23.003 clause 28.7.2).
- A reader of octets is canonical: it gives a value only for the octets that
  the value is written as. The readers named `_tolerant` also take what other
  implementations are known to send, with a list of what was not by the
  clause, which allocates nothing: another end mark than `1111` after an even
  number of identity digits, the spare bit of the first octet of a 5G-GUTI or
  a 5G-S-TMSI, the spare bits and the spare SUPI formats of a SUCI, and the
  scheme output of a SUCI of an IMSI of another length than its profile gives
  it, which only the home network reads, and refuses.
- `tree`, with the feature `tree`: the tree of a message as a `tree::Value`,
  JSON whose objects keep their members in the order in which they were
  written, which `tree!` writes in the code as `serde_json::json!` does. It
  converts to and from a `serde_json::Value`. Of the features of
  `serde_json`, the crate turns on `float_roundtrip` alone: the JSON of a
  program that depends on it keeps the order that the program chose, and a
  number with a fraction that a tree prints is read back as it was. An
  object of more than 32 members keeps an index of its names, so that one is
  found in a time that grows as log n. A name written twice in an object of JSON is
  refused. `Tree`, `Members` and `Reading` write a type as a tree and read it
  from one, with refusals that say the member and why, and nothing of what
  is written in `Reading::secret`, where a key is read. `Reading::named`
  reads `true` and `false` as those names of a type that has them, as YAML
  leaves them unquoted, and refuses another name of a type of more than 12
  with the nearest of them (`tree::nearest`), or its first. Octets are
  hexadecimal in lower case, read in either; a number is a JSON number; a
  name is read by its letters and its digits, however it is written.
- Paths on a tree: `select`, `paths`, `set`, `remove` and `insert`, with
  4096 octets, 64 segments and 65,536 selected values at most (the longest
  list of the protocols, the connections of an NG RESET), one error type
  with a kind, and edits that are done whole or not at all, in the place of
  the tree: what an edit changes is kept and put back when a later place
  cannot take it, and nothing of the tree is copied. `Family` is what a
  family of messages adds where its trees are more than objects and lists;
  the functions that end with `_in` take one, `places_in` gives the places of
  a path with what the family knows of each, and `open_in` shows the values
  on the way of a path as the family opens them, every place of a segment in
  one walk of the tree. `null` is read as absent, but where the family says
  that the type of the place is NULL (`Family::null_is_value`): an
  alternative `{"pLMNWide": null}` is then a value that a path lists and
  selects. `Root`, `Edit`, `Key`, `Step`, `Absent`, `Listed` and `Removal`
  are `#[non_exhaustive]`, and `Root::new` makes a root. A family refuses an
  edit in its step, before anything changes, takes a segment in a member with
  its own rule there (`Next::through`), and gives its errors a code of its
  own.
  `written_as` brings a value that someone wrote to the form that a tree
  shows at a place, for a comparison.
- `tree::serde`, with the feature `serde`: `Serialize` and `Deserialize` for
  a type through its tree, by `impl_serde!` or the two functions that it
  calls. `impl_serde!` takes a type with a lifetime as `Name<'a>`, which is
  deserialized with any lifetime.
- `no_std`, no `unsafe`, no dependency without a feature. What holds a text
  or a list is the feature `alloc`, which `tree` and `idna` turn on: without
  it the crate needs no allocator, so that a program without one can take the
  PLMN, the GUTIs and the other values that have their digits in place. The
  minimum supported Rust version is 1.88.
