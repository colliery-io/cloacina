#  Copyright 2025-2026 Colliery Software
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.

"""Discover ``@task`` decorated functions from Python source modules.

This performs static AST-based discovery so we don't need to import
user code (which may have unresolved dependencies at build time).
"""

from __future__ import annotations

import ast
from pathlib import Path

from cloaca.manifest import TaskDefinition


def discover_tasks(entry_module: str, project_dir: Path) -> list[TaskDefinition]:
    """Discover tasks from *entry_module* under *project_dir*.

    Scans for functions decorated with ``@task`` and extracts metadata
    from decorator keyword arguments.
    """
    module_path = _resolve_module_path(entry_module, project_dir)
    if not module_path.exists():
        raise FileNotFoundError(
            f"Entry module not found: {module_path} "
            f"(from entry_module='{entry_module}')"
        )

    source = module_path.read_text(encoding="utf-8")
    tree = ast.parse(source, filename=str(module_path))

    tasks: list[TaskDefinition] = []
    for node in ast.walk(tree):
        if not isinstance(node, ast.FunctionDef):
            continue

        task_kwargs = _extract_task_decorator(node)
        if task_kwargs is None:
            continue

        task_id = task_kwargs.get("id", node.name)
        function_path = f"{entry_module}:{node.name}"

        deps_node = task_kwargs.get("dependencies")
        dependencies: list[str] = []
        if isinstance(deps_node, list):
            dependencies = deps_node

        tasks.append(
            TaskDefinition(
                id=task_id,
                function=function_path,
                dependencies=dependencies,
                description=task_kwargs.get("description"),
                retries=int(task_kwargs.get("retries", 0)),
                timeout_seconds=task_kwargs.get("timeout_seconds"),
            )
        )

    return tasks


def _resolve_module_path(dotted: str, project_dir: Path) -> Path:
    """Convert a dotted module path to a filesystem path."""
    parts = dotted.split(".")
    # Try as a file first: workflow.tasks -> workflow/tasks.py
    file_path = project_dir / Path(*parts).with_suffix(".py")
    if file_path.exists():
        return file_path
    # Try as a package: workflow.tasks -> workflow/tasks/__init__.py
    pkg_path = project_dir / Path(*parts) / "__init__.py"
    return pkg_path if pkg_path.exists() else file_path


def _extract_task_decorator(func: ast.FunctionDef) -> dict | None:
    """Return keyword args if *func* has a ``@task`` decorator, else ``None``."""
    for decorator in func.decorator_list:
        name = None
        kwargs: dict = {}

        if isinstance(decorator, ast.Name) and decorator.id == "task":
            return kwargs
        if isinstance(decorator, ast.Call):
            if isinstance(decorator.func, ast.Name) and decorator.func.id == "task":
                name = "task"
            elif (
                isinstance(decorator.func, ast.Attribute)
                and decorator.func.attr == "task"
            ):
                name = "task"

            if name == "task":
                for kw in decorator.keywords:
                    if kw.arg is not None:
                        kwargs[kw.arg] = _eval_literal(kw.value)
                return kwargs

    return None


def _eval_literal(node: ast.expr) -> object:
    """Safely evaluate an AST node as a literal value."""
    try:
        return ast.literal_eval(node)
    except (ValueError, TypeError):
        return None
