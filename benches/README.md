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
on the same JSON, and the identities, at 8865ad4, on core 2 of an AMD Ryzen 7
5700G (16 threads, Linux 7.0, Rust 1.98.1), on a machine that ran nothing
else, on 2026-10-18. A time is the lowest of five runs of the benchmark, and
the allocations are those of one call.

Against `serde_json::Value`:

| Operation | This crate | Allocations | Octets allocated | serde_json 1 | Allocations | Octets allocated | serde_json |
|---|---|---|---|---|---|---|---|
| Read a tree of 925 octets | 2,645 ns | 93 | 6,618 | 2,880 ns | 72 | 14,594 | about the same |
| Write it | 923 ns | 5 | 2,845 | 2,140 ns | 8 | 2,040 | 2.3× slower |
| Select a value of it | 219 ns | 10 | 385 | 238 ns | 8 | 64 | about the same |
| Put 4096 members into an object, one after the other | 15.9 ms | 4,107 | 477,898 | 0.65 ms | 4,731 | 429,138 | 24× faster |
| Read an object of 4096 members, 39,851 octets | 380 µs | 4,108 | 543,434 | 774 µs | 4,731 | 429,138 | 2.0× slower |

The time of writing a tree moves by 15 % between builds of the same code.

The crate alone:

| Operation | Time | Allocations | Octets allocated |
|---|---|---|---|
| Push 4096 new members into an object, one after the other | 145 µs | 4,107 | 477,898 |
| Select a member that half of the 65,535 entries of a list lack | 1.48 ms | 34 | 4,718,555 |
| Copy a list of 65,535 entries | 7.75 ms | 262,140 | 8,126,308 |
| Copy it and set the member in every entry | 45.0 ms | 884,789 | 54,131,789 |
| Copy it and remove every other entry | 11.3 ms | 327,724 | 18,088,005 |
| Copy it and insert an entry before each one | 15.2 ms | 393,262 | 30,670,701 |
| Read a PLMN from text | 14 ns | 0 | 0 |
| Read an IMSI from text | 9 ns | 0 | 0 |
| Read a 5G-GUTI from a 5GS mobile identity | 14 ns | 0 | 0 |
| Read the SUCI of an IMSI from a 5GS mobile identity | 80 ns | 2 | 20 |
| Write a PLMN in the order of NGAP | 7 ns | 0 | 0 |

The layout of the code moves a time by up to 15 % from one build of the same
code to another: a ratio within 10 %, or within the spread of the runs, is
about the same. A tree keeps the members of an object in their order, where a
`serde_json::Value` sorts them. An object is a list that is searched for a
name: putting n members one after the other takes a time that grows as n²,
and `Map::push` adds a member whose name is new in a time that grows as n.
The identities allocate nothing but the SUCI, which holds its scheme output.

Some rows were taken later, on the same machine: the rows of 4096 members
that are put and read on 2026-11-07, the row that pushes them on 2026-11-13
(Rust 1.88.0, while the machine ran other work: put took 18.0 ms there), and
the five rows of 65,535 entries on 2026-11-11, while the machine ran other
work. Those five use a path family that behaves like those of the protocol
crates: list entries are selected by a name that they hold.
