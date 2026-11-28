"""
Priority on a Python workflow and task (CLOACINA-T-1123).

A workflow sets its priority with `builder.priority(n)` and a task with
`@cloaca.task(priority=n)`; both are read back from the rows of the run. A
value outside 0 to 10 raises ValueError.

The run uses the database of the backend under test (`CLOACA_BACKEND`), and
the test reads the rows with `sqlite3` or `psycopg2`.
"""

import sqlite3

import pytest


def _rows(db_url, sql, params):
    """Rows of one query; `sql` uses `?` placeholders."""
    if db_url.startswith("sqlite://"):
        conn = sqlite3.connect(db_url.split("://", 1)[1])
        try:
            return conn.execute(sql, params).fetchall()
        finally:
            conn.close()
    import psycopg2

    conn = psycopg2.connect(db_url)
    try:
        with conn.cursor() as cur:
            cur.execute(sql.replace("?", "%s"), params)
            return cur.fetchall()
    finally:
        conn.close()


class TestPriority:
    """Workflow and task priority from Python."""

    def test_workflow_and_task_priority_are_on_the_rows_of_the_run(self, run_db_url):
        import cloaca

        runner = cloaca.DefaultRunner(run_db_url)
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

            execution = _rows(
                run_db_url,
                "SELECT priority_asked, priority FROM workflow_executions "
                "WHERE workflow_name = ?",
                ("priority_scenario_workflow",),
            )
            assert [tuple(row) for row in execution] == [(2, 2)]

            tasks = dict(
                (name.rsplit("::", 1)[-1], (workflow, own, asked))
                for name, workflow, own, asked in _rows(
                    run_db_url,
                    "SELECT t.task_name, t.workflow_priority, t.task_priority, "
                    "t.task_priority_asked FROM task_executions t "
                    "JOIN workflow_executions w ON w.id = t.workflow_execution_id "
                    "WHERE w.workflow_name = ?",
                    ("priority_scenario_workflow",),
                )
            )
            assert tasks["urgent"] == (2, 0, 0)
            assert tasks["plain"] == (2, 2, None)
        finally:
            runner.shutdown()

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
