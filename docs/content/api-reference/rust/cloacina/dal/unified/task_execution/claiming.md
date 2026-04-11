# cloacina::dal::unified::task_execution::claiming <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task claiming and retry scheduling operations.

All operations are transactional: state changes and execution events
are written atomically. If either fails, both are rolled back.
