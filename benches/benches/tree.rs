/*
   OxiRush
   Copyright 2025-2026 Valentin D'Emmanuele

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! The time of each operation, and its CPU profile: `group/implementation`,
//! as `read-tree/serde_json`. `src/lib.rs` has the commands.

use criterion::{Criterion, criterion_group, criterion_main};
use oxirush_utils_benches::operations;
use pprof::criterion::{Output, PProfProfiler};
use std::time::Duration;

fn tree(criterion: &mut Criterion) {
    for mut operation in operations() {
        let mut group = criterion.benchmark_group(operation.group);
        // A call of a few milliseconds is sampled ten times
        if operation.long {
            group.sample_size(10);
        }
        group.bench_function(operation.implementation, |bencher| {
            bencher.iter(&mut operation.call)
        });
        group.finish();
    }
}

fn configuration() -> Criterion {
    let output = match std::env::var("PROFILE").as_deref() {
        Ok("flamegraph") => Output::Flamegraph(None),
        _ => Output::Protobuf,
    };
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2))
        .with_profiler(PProfProfiler::new(1000, output))
}

criterion_group! {
    name = benches;
    config = configuration();
    targets = tree
}
criterion_main!(benches);
