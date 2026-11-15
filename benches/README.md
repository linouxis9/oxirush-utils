# Benchmarks of oxirush-utils

A package of its own, which the published crate does not have. `tree` times
the tree and the identities with criterion, and `allocations` counts the
allocations of one call; [`src/lib.rs`](src/lib.rs) says how to profile one
of them. The [README](../README.md#performance) sums the figures up. In this
directory:

```sh
cargo bench --bench tree          # the times, with criterion
cargo bench --bench allocations   # the allocations of one call
```

## Every figure

The tree of the crate against a `serde_json::Value` of the default features,
on the same JSON, and the identities, at 824b57f, on core 2 of an Intel Xeon
E5-2680 v4 (Broadwell, 2.4 GHz; a KVM guest bound to NUMA node 0; Linux 7.0,
Rust 1.100.0), on 2026-11-15. A time is the median of ten runs of the
benchmark, the two implementations in turns, each run the median of
criterion's samples; the ten runs of a row were within 16 % of each other.
The allocations are those of one call.

Against `serde_json::Value`:

| Operation | This crate | Allocations | Octets allocated | serde_json 1 | Allocations | Octets allocated | serde_json |
|---|---|---|---|---|---|---|---|
| Read a tree of 925 octets | 5,046 ns | 93 | 6,618 | 6,240 ns | 72 | 14,594 | 1.2× slower |
| Write it | 1,597 ns | 5 | 2,845 | 4,139 ns | 8 | 2,040 | 2.6× slower |
| Select a value of it | 425 ns | 10 | 521 | 473 ns | 8 | 64 | 1.1× slower |
| Put 4096 members into an object, one after the other | 25.5 ms | 4,107 | 477,898 | 1.32 ms | 4,731 | 429,138 | 19× faster |
| Read an object of 4096 members, 39,851 octets | 833 µs | 4,108 | 543,434 | 1,478 µs | 4,731 | 429,138 | 1.8× slower |

The time of writing a tree moves by 15 % between builds of the same code.

The crate alone:

| Operation | Time | Allocations | Octets allocated |
|---|---|---|---|
| Push 4096 new members into an object, one after the other | 330 µs | 4,107 | 477,898 |
| Select a member that half of the 65,535 entries of a list lack | 2.61 ms | 34 | 4,718,555 |
| Copy a list of 65,535 entries | 5.69 ms | 65,536 | 7,602,032 |
| Copy it and set the member in every entry | 55.4 ms | 622,648 | 53,214,291 |
| Copy it and remove every other entry | 13.0 ms | 131,120 | 17,563,729 |
| Copy it and insert an entry before each one | 22.3 ms | 196,658 | 30,146,425 |
| Read a PLMN from text | 25 ns | 0 | 0 |
| Read an IMSI from text | 20 ns | 0 | 0 |
| Read a 5G-GUTI from a 5GS mobile identity | 27 ns | 0 | 0 |
| Read the SUCI of an IMSI from a 5GS mobile identity | 132 ns | 2 | 20 |
| Write a PLMN in the order of NGAP | 9 ns | 0 | 0 |

The layout of the code moves a time by up to 15 % from one build of the same
code to another: "about the same" is a ratio within 10 %, or two sets of
runs that overlap. A tree keeps the members of an object in their order, where a
`serde_json::Value` sorts them. An object is a list that is searched for a
name: putting n members one after the other takes a time that grows as n²,
and `Map::push` adds a member whose name is new in a time that grows as n.
The identities allocate nothing but the SUCI, which holds its scheme output.
