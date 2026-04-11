# cloacina::dispatcher::traits <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Trait definitions for dispatcher and executor abstractions.

This module defines the core traits that enable pluggable executor backends.
Implementors can create custom executors (Kubernetes, serverless, message queues)
that integrate seamlessly with the scheduler.
