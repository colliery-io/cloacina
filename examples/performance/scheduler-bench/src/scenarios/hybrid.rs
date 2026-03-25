/*
 *  Copyright 2025-2026 Colliery Software
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

//! Hybrid benchmarks — all schedulers simultaneously (T-0254).

use crate::metrics::BenchmarkResult;
use std::time::Duration;

pub async fn run(
    _db: &cloacina::Database,
    _duration: Duration,
    _scenario: Option<&str>,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    Err("Hybrid benchmarks not yet implemented (T-0254)".into())
}
