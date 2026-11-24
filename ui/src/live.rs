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

//! Live delivery streams that end with their effect (CLOACINA-T-0952).
//!
//! A stream loop checks its generation only when a frame arrives. A stream
//! that gets no more frames (an execution that ended, a page the user left)
//! never sees the check, and its socket stays open. Spawn such loops with
//! [`spawn_until_cleanup`]: when the effect re-runs or its owner is disposed,
//! the loop is aborted and dropped, and dropping the stream closes the socket.

use std::future::Future;

use futures_util::future::{AbortHandle, Abortable};
use leptos::prelude::on_cleanup;

/// Wrap `fut` so that it is aborted when the current reactive owner cleans
/// up. Call it inside the effect that starts the stream.
pub fn abort_on_cleanup<F: Future>(fut: F) -> Abortable<F> {
    let (handle, registration) = AbortHandle::new_pair();
    on_cleanup(move || handle.abort());
    Abortable::new(fut, registration)
}

/// Spawn `fut` on the local executor; abort and drop it when the current
/// reactive owner cleans up.
pub fn spawn_until_cleanup(fut: impl Future<Output = ()> + 'static) {
    let fut = abort_on_cleanup(fut);
    leptos::task::spawn_local(async move {
        let _ = fut.await;
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use futures_util::FutureExt;
    use leptos::prelude::Owner;

    use super::abort_on_cleanup;

    /// Sets its flag when dropped, like the socket that closes on drop.
    struct Socket(Arc<AtomicBool>);
    impl Drop for Socket {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn cleanup_aborts_a_silent_stream_and_drops_its_socket() {
        let closed = Arc::new(AtomicBool::new(false));
        let socket = Socket(closed.clone());
        let owner = Owner::new();
        // A loop that never gets another frame.
        let mut fut = owner.with(|| {
            Box::pin(abort_on_cleanup(async move {
                let _socket = socket;
                futures_util::future::pending::<()>().await;
            }))
        });

        assert!(fut.as_mut().now_or_never().is_none(), "still waiting");
        assert!(!closed.load(Ordering::SeqCst));

        owner.cleanup(); // the effect re-runs, or the page leaves
        let out = fut.as_mut().now_or_never().expect("aborted");
        assert!(out.is_err());
        drop(fut);
        assert!(closed.load(Ordering::SeqCst), "socket closed");
    }

    #[test]
    fn without_cleanup_the_stream_keeps_running() {
        let owner = Owner::new();
        let fut = owner.with(|| abort_on_cleanup(async { 7 }));
        assert_eq!(fut.now_or_never(), Some(Ok(7)));
    }
}
