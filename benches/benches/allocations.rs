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

//! The allocations of one call of each operation, after a first call that is
//! not counted, and the octets that they ask for. The numbers are exact, and
//! the same on every run: an allocator that counts stands in for the system
//! allocator in this program, which makes no measure of time.

use oxirush_utils_benches::operations;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// An allocator that counts.
struct Counting;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static OCTETS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        OCTETS.fetch_add(layout.size(), Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Relaxed);
        OCTETS.fetch_add(size, Relaxed);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn main() {
    println!(
        "{:<20} {:<14} {:>12} {:>12}",
        "operation", "of", "allocations", "octets"
    );
    for mut operation in operations() {
        (operation.call)();
        let before = (ALLOCATIONS.load(Relaxed), OCTETS.load(Relaxed));
        (operation.call)();
        let (allocations, octets) = (
            ALLOCATIONS.load(Relaxed) - before.0,
            OCTETS.load(Relaxed) - before.1,
        );
        println!(
            "{:<20} {:<14} {allocations:>12} {octets:>12}",
            operation.group, operation.implementation
        );
    }
}
