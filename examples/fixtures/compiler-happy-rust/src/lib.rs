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

// Minimal cdylib — a single exported symbol so cargo has something to link.
// The compiler service only cares that `cargo build --release --lib` succeeds
// and produces a .dylib/.so; it does not validate fidius FFI contracts here.
#[no_mangle]
pub extern "C" fn cloacina_compiler_e2e_noop() {}
