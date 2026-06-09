# cloacina::computation_graph::triggerless <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Trigger-less computation graph registration.

T-0552 (I-0102 follow-up) relocated `TriggerlessGraphFn`,
`TriggerlessGraphRegistration`, and the `TriggerlessGraph` trait into
`cloacina-workflow-plugin` so packaged cdylibs can collect
`TriggerlessGraphEntry` inventory entries at link time. Engine paths
re-export.
