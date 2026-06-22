from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.input_slot import InputSlot
    from ..models.workflow_task_node import WorkflowTaskNode


T = TypeVar("T", bound="WorkflowDetail")


@_attrs_define
class WorkflowDetail:
    """`GET /tenants/{tenant_id}/workflows/{name}` response — summary fields
    plus real build state (pending/building/failed/success).

        Attributes:
            build_status (str):
            created_at (str): RFC 3339 timestamp.
            id (str): Package UUID.
            package_name (str):
            tasks (list[str]): Task IDs included in this package.
            tenant_id (str):
            version (str):
            workflow_name (str): Executable workflow name (the identifier to execute by). Differs from
                `package_name` under the standard convention (package `demo-slow-rust`
                → workflow `demo_slow_workflow`). Falls back to `package_name` for
                packages predating workflow-name persistence. (CLOACI-T-0671)
            build_error (None | str | Unset):
            declared_params (list[InputSlot] | Unset): CLOACI-I-0128: declared input params (named, JSON-Schema-typed slots)
                the
                workflow accepts at execute time. Empty when undeclared. Lets the UI
                render a typed execute form and the server validate context.
            description (None | str | Unset):
            paused (bool | Unset): Whether this workflow is paused (CLOACI-T-0749). Paused workflows refuse
                new executions until resumed.
            task_graph (list[WorkflowTaskNode] | Unset): The task dependency graph (nodes + their upstream dependencies) for
                rendering the full workflow DAG. Empty for packages predating
                task-graph persistence. (CLOACI-T-0663)
    """

    build_status: str
    created_at: str
    id: str
    package_name: str
    tasks: list[str]
    tenant_id: str
    version: str
    workflow_name: str
    build_error: None | str | Unset = UNSET
    declared_params: list[InputSlot] | Unset = UNSET
    description: None | str | Unset = UNSET
    paused: bool | Unset = UNSET
    task_graph: list[WorkflowTaskNode] | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        build_status = self.build_status

        created_at = self.created_at

        id = self.id

        package_name = self.package_name

        tasks = self.tasks

        tenant_id = self.tenant_id

        version = self.version

        workflow_name = self.workflow_name

        build_error: None | str | Unset
        if isinstance(self.build_error, Unset):
            build_error = UNSET
        else:
            build_error = self.build_error

        declared_params: list[dict[str, Any]] | Unset = UNSET
        if not isinstance(self.declared_params, Unset):
            declared_params = []
            for declared_params_item_data in self.declared_params:
                declared_params_item = declared_params_item_data.to_dict()
                declared_params.append(declared_params_item)

        description: None | str | Unset
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        paused = self.paused

        task_graph: list[dict[str, Any]] | Unset = UNSET
        if not isinstance(self.task_graph, Unset):
            task_graph = []
            for task_graph_item_data in self.task_graph:
                task_graph_item = task_graph_item_data.to_dict()
                task_graph.append(task_graph_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "build_status": build_status,
                "created_at": created_at,
                "id": id,
                "package_name": package_name,
                "tasks": tasks,
                "tenant_id": tenant_id,
                "version": version,
                "workflow_name": workflow_name,
            }
        )
        if build_error is not UNSET:
            field_dict["build_error"] = build_error
        if declared_params is not UNSET:
            field_dict["declared_params"] = declared_params
        if description is not UNSET:
            field_dict["description"] = description
        if paused is not UNSET:
            field_dict["paused"] = paused
        if task_graph is not UNSET:
            field_dict["task_graph"] = task_graph

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.input_slot import InputSlot
        from ..models.workflow_task_node import WorkflowTaskNode

        d = dict(src_dict)
        build_status = d.pop("build_status")

        created_at = d.pop("created_at")

        id = d.pop("id")

        package_name = d.pop("package_name")

        tasks = cast(list[str], d.pop("tasks"))

        tenant_id = d.pop("tenant_id")

        version = d.pop("version")

        workflow_name = d.pop("workflow_name")

        def _parse_build_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        build_error = _parse_build_error(d.pop("build_error", UNSET))

        _declared_params = d.pop("declared_params", UNSET)
        declared_params: list[InputSlot] | Unset = UNSET
        if _declared_params is not UNSET:
            declared_params = []
            for declared_params_item_data in _declared_params:
                declared_params_item = InputSlot.from_dict(declared_params_item_data)

                declared_params.append(declared_params_item)

        def _parse_description(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        description = _parse_description(d.pop("description", UNSET))

        paused = d.pop("paused", UNSET)

        _task_graph = d.pop("task_graph", UNSET)
        task_graph: list[WorkflowTaskNode] | Unset = UNSET
        if _task_graph is not UNSET:
            task_graph = []
            for task_graph_item_data in _task_graph:
                task_graph_item = WorkflowTaskNode.from_dict(task_graph_item_data)

                task_graph.append(task_graph_item)

        workflow_detail = cls(
            build_status=build_status,
            created_at=created_at,
            id=id,
            package_name=package_name,
            tasks=tasks,
            tenant_id=tenant_id,
            version=version,
            workflow_name=workflow_name,
            build_error=build_error,
            declared_params=declared_params,
            description=description,
            paused=paused,
            task_graph=task_graph,
        )

        workflow_detail.additional_properties = d
        return workflow_detail

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
