"""
Priority on a Python workflow and task (CLOACINA-T-1123).

A workflow sets its priority with `builder.priority(n)` and a task with
`@cloaca.task(priority=n)`; both are read back from the rows of the run. A
value outside 0 to 10 raises ValueError.

The run uses its own SQLite database, so the test can read the rows with
the `sqlite3` module.
"""

import os
import sqlite3
import tempfile

import pytest


class TestPriority:
    """Workflow and task priority from Python."""

    def test_workflow_and_task_priority_are_on_the_rows_of_the_run(self):
        import cloaca

        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as tmp:
            db_path = tmp.name
        runner = cloaca.DefaultRunner(f"sqlite://{db_path}")
        try:
            with cloaca.WorkflowBuilder("priority_scenario_workflow") as builder:
                builder.priority(2)

                @cloaca.task(id="urgent", priority=0)
                def urgent(context):
                    context.set("urgent", True)
                    return context

                @cloaca.task(id="plain")
                def plain(context):
                    context.set("plain", True)
                    return context

            result = runner.execute("priority_scenario_workflow", cloaca.Context({}))
            assert result.status == "Completed"

            conn = sqlite3.connect(db_path)
            try:
                execution = conn.execute(
                    "SELECT priority_asked, priority FROM workflow_executions "
                    "WHERE workflow_name = ?",
                    ("priority_scenario_workflow",),
                ).fetchall()
                assert execution == [(2, 2)]

                tasks = dict(
                    (name.rsplit("::", 1)[-1], (workflow, own, asked))
                    for name, workflow, own, asked in conn.execute(
                        "SELECT task_name, workflow_priority, task_priority, "
                        "task_priority_asked FROM task_executions"
                    )
                )
            finally:
                conn.close()
            assert tasks["urgent"] == (2, 0, 0)
            assert tasks["plain"] == (2, 2, None)
        finally:
            runner.shutdown()
            if os.path.exists(db_path):
                os.unlink(db_path)

    def test_a_priority_outside_the_scale_raises_value_error(self):
        import cloaca

        with pytest.raises(ValueError):
            cloaca.task(id="above_ten", priority=11)
        with pytest.raises(ValueError):
            cloaca.task(id="negative", priority=-1)

        builder = cloaca.WorkflowBuilder("priority_scenario_bad_workflow")
        with pytest.raises(ValueError):
            builder.priority(11)
        with pytest.raises(ValueError):
            builder.priority(-1)
        builder.priority(0)
        builder.priority(10)
