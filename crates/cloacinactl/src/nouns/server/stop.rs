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

use anyhow::Result;

use crate::shared::pid;
use crate::GlobalOpts;

pub async fn run(globals: &GlobalOpts, force: bool) -> Result<()> {
    let pid_path = globals.home.join("server.pid");
    let Some(p) = pid::try_read(&pid_path) else {
        // Per spec: stub with clear message; return 0 so CI scripts don't break.
        println!(
            "no local server PID file at {} — stop via your orchestrator (Docker, K8s, systemd)",
            pid_path.display()
        );
        return Ok(());
    };

    pid::signal_and_wait(p, force, std::time::Duration::from_secs(10))?;
    let _ = pid::remove(&pid_path);
    println!("server stopped (pid {p})");
    Ok(())
}
