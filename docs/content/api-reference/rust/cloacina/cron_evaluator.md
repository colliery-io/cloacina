# cloacina::cron_evaluator <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


T-0553 / I-0102: timezone-aware cron expression evaluator relocated to `cloacina-workflow` so packaged cdylibs (which depend on `cloacina-workflow` but not on `cloacina`) can use it from the `#[trigger(cron = "...")]` macro emission. Engine paths re-export.
