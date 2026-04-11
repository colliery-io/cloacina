# cloacina::dal::unified::task_execution::state <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


State transition operations for task executions.

All state transitions are transactional: the status update and execution event
are written atomically. If either fails, both are rolled back.
