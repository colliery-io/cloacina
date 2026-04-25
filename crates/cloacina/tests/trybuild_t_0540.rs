/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! T-0540 compile-failure regression tests.
//!
//! These use `trybuild` to assert that misuse of `#[task(invokes =
//! computation_graph(...))]` produces a useful compile error. Because the
//! macro's compile-time gate is "the referenced graph must implement
//! `TriggerlessGraph`," the rustc diagnostic is what users see — we lock
//! down its content so future macro refactors can't silently weaken the
//! contract.
//!
//! Stderr fixtures are written to `tests/trybuild_t_0540/*.stderr` and are
//! regenerated on first run if missing (see trybuild docs).

#[test]
fn t_0540_compile_failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild_t_0540/invokes_reactor_triggered.rs");
}
