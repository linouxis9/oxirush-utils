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

| Operation | this crate | serde_json 1 | serde_json / this crate |
|---|---|---|---|
| Read a tree of 925 octets | 2,645 ns, 93 allocations of 6,618 octets | 2,880 ns, 72 allocations of 14,594 octets | about the same |
| Write it | 923 ns ±15% between builds, 5 allocations of 2,845 octets | 2,140 ns, 8 allocations of 2,040 octets | 2.3x |
| Select a value of it | 219 ns, 10 allocations of 385 octets | 238 ns, 8 allocations of 64 octets | about the same |
| Put 100,000 members into an object | 21.0 ms, 113,368 allocations of 17,990,994 octets | 18.6 ms, 115,460 allocations of 10,564,762 octets | 0.89x |
| Read an object of 100,000 members, 1,088,891 octets | 14.6 ms, 109,113 allocations of 21,887,474 octets | 20.7 ms, 115,460 allocations of 10,564,762 octets | 1.4x |
| Read a PLMN from text | 14 ns, 0 allocations of 0 octets | | |
| Read an IMSI from text | 9 ns, 0 allocations of 0 octets | | |
| Read a 5G-GUTI from a 5GS mobile identity | 14 ns, 0 allocations of 0 octets | | |
| Read the SUCI of an IMSI from a 5GS mobile identity | 80 ns, 2 allocations of 20 octets | | |
| Write a PLMN in the order of NGAP | 7 ns, 0 allocations of 0 octets | | |

The layout of the code moves a time by up to 15% from one build of the same
code to another: a ratio within 10%, or within the spread of the runs, is
about the same. A tree is read and written with serde_json, and keeps the
members of an object in their order, where a `serde_json::Value` sorts them.
An object of more than 256 members keeps an index of the hashes of its names,
without a copy of a name: reading n members takes a time that grows as
n log n. A smaller object, as every value of a protocol, is a list that is
searched. The identities allocate nothing but the SUCI, which holds its scheme
output.
