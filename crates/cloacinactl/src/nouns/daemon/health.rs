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
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;

use crate::GlobalOpts;

pub async fn run(globals: &GlobalOpts) -> Result<()> {
    let socket_path = globals.home.join("daemon.sock");
    if !socket_path.exists() {
        eprintln!("down");
        std::process::exit(2);
    }

    match UnixStream::connect(&socket_path).await {
        Ok(mut stream) => {
            let mut buf = Vec::new();
            if stream.read_to_end(&mut buf).await.is_err() {
                eprintln!("down");
                std::process::exit(2);
            }
            println!("up");
            Ok(())
        }
        Err(_) => {
            eprintln!("down");
            std::process::exit(2);
        }
    }
}
