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
| Put 4096 members into an object, one after the other | 15.9 ms, 4,107 allocations of 477,898 octets | 0.65 ms, 4,731 allocations of 429,138 octets | 0.04x |
| Read an object of 4096 members, 39,851 octets | 380 µs, 4,108 allocations of 543,434 octets | 774 µs, 4,731 allocations of 429,138 octets | 2.0x |
| Select a member that half of the 65,535 entries of a list lack | 1.48 ms, 34 allocations of 4,718,555 octets | | |
| Copy a list of 65,535 entries | 7.75 ms, 262,140 allocations of 8,126,308 octets | | |
| Copy it and set the member in every entry | 45.0 ms, 884,789 allocations of 54,131,789 octets | | |
| Copy it and remove every other entry | 11.3 ms, 327,724 allocations of 18,088,005 octets | | |
| Copy it and insert an entry before each one | 15.2 ms, 393,262 allocations of 30,670,701 octets | | |
| Read a PLMN from text | 14 ns, 0 allocations of 0 octets | | |
| Read an IMSI from text | 9 ns, 0 allocations of 0 octets | | |
| Read a 5G-GUTI from a 5GS mobile identity | 14 ns, 0 allocations of 0 octets | | |
| Read the SUCI of an IMSI from a 5GS mobile identity | 80 ns, 2 allocations of 20 octets | | |
| Write a PLMN in the order of NGAP | 7 ns, 0 allocations of 0 octets | | |

The layout of the code moves a time by up to 15% from one build of the same
code to another: a ratio within 10%, or within the spread of the runs, is
about the same. A tree is read and written with serde_json, and keeps the
members of an object in their order, where a `serde_json::Value` sorts them.
An object is a list that is searched for a name: putting n members one after
the other takes a time that grows as n², which the few dozen members of a
value of a protocol never show, and an object that is read has 4096 members
at most. The two rows of 4096 members were measured on 2026-11-07, when an
object became a list; the others at 8865ad4. The identities allocate nothing but the SUCI, which holds its scheme
output.

The five rows of 65,535 entries use a path family that behaves like those of
the protocol crates: list entries are selected by a name they hold, and a
member that an entry lacks is a valueless place, added once per type in a
selection and once per entry in an edit. They were measured on 2026-11-11 on
the same machine, while it was running other work, after two quadratic steps
of the path engine were made linear. Before that change the selection took
315 ms, the copy with every other entry removed 200 ms, and the copy with an
entry inserted before each one 1.70 s. The row that sets the member was added
when `set` came to give it to every entry that lacks it: before, it gave it to
the entries that have it and to the first that lacks it, 32,768 of them, in
20.5 ms.
