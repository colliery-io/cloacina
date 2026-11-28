from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.input_slot import InputSlot
    from ..models.needs_new_build import NeedsNewBuild
    from ..models.priority_grant import PriorityGrant
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
            describe_error (None | str | Unset): Why the compiler could not write the metadata document of this built
                package (CLOACINA-T-1074). Set for a package that was built before the
                document existed and cannot be described from its stored build.
                Absent when the package is described, or has not been tried yet. A
                package whose describe fails during a build has `build_status`
                `failed` and the reason in `build_error`.
            description (None | str | Unset):
            needs_new_build (NeedsNewBuild | None | Unset):
            not_loaded_reason (None | str | Unset): Why this built package is not loaded (CLOACINA-T-1075). A server that
                loads packages from their metadata only does not load a package that
                has no metadata document: it waits for the tenant's compiler to
                describe it, or the compiler could not (`describe_error`). Absent when
                the package has a document, is not built, or the server runs with
                single trust. A package the server refuses has `build_status`
                `failed` and the reason in `build_error`.
            paused (bool | Unset): Whether this workflow is paused (CLOACI-T-0749). Paused workflows refuse
                new executions until resumed.
            plugin_interface_version (int | None | Unset): As in [`WorkflowSummary::plugin_interface_version`]
                (CLOACINA-T-1085).
            priority (None | PriorityGrant | Unset):
            priority_notice (None | str | Unset): Set when `given` is lower than `asked`: what was asked, what is
                given, and why.
            task_graph (list[WorkflowTaskNode] | Unset): The task dependency graph (nodes + their upstream dependencies) for
                rendering the full workflow DAG. Empty for packages predating
                task-graph persistence. (CLOACI-T-0663)
            uploaded_by (None | str | Unset): Who uploaded the package (CLOACINA-T-1024): the login identity, or
                the name of the API key. Read from the audit record; `null` for a
                package uploaded before the audit record existed.
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
    describe_error: None | str | Unset = UNSET
    description: None | str | Unset = UNSET
    needs_new_build: NeedsNewBuild | None | Unset = UNSET
    not_loaded_reason: None | str | Unset = UNSET
    paused: bool | Unset = UNSET
    plugin_interface_version: int | None | Unset = UNSET
    priority: None | PriorityGrant | Unset = UNSET
    priority_notice: None | str | Unset = UNSET
    task_graph: list[WorkflowTaskNode] | Unset = UNSET
    uploaded_by: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.needs_new_build import NeedsNewBuild
        from ..models.priority_grant import PriorityGrant

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

        describe_error: None | str | Unset
        if isinstance(self.describe_error, Unset):
            describe_error = UNSET
        else:
            describe_error = self.describe_error

        description: None | str | Unset
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        needs_new_build: dict[str, Any] | None | Unset
        if isinstance(self.needs_new_build, Unset):
            needs_new_build = UNSET
        elif isinstance(self.needs_new_build, NeedsNewBuild):
            needs_new_build = self.needs_new_build.to_dict()
        else:
            needs_new_build = self.needs_new_build

        not_loaded_reason: None | str | Unset
        if isinstance(self.not_loaded_reason, Unset):
            not_loaded_reason = UNSET
        else:
            not_loaded_reason = self.not_loaded_reason

        paused = self.paused

        plugin_interface_version: int | None | Unset
        if isinstance(self.plugin_interface_version, Unset):
            plugin_interface_version = UNSET
        else:
            plugin_interface_version = self.plugin_interface_version

        priority: dict[str, Any] | None | Unset
        if isinstance(self.priority, Unset):
            priority = UNSET
        elif isinstance(self.priority, PriorityGrant):
            priority = self.priority.to_dict()
        else:
            priority = self.priority

        priority_notice: None | str | Unset
        if isinstance(self.priority_notice, Unset):
            priority_notice = UNSET
        else:
            priority_notice = self.priority_notice

        task_graph: list[dict[str, Any]] | Unset = UNSET
        if not isinstance(self.task_graph, Unset):
            task_graph = []
            for task_graph_item_data in self.task_graph:
                task_graph_item = task_graph_item_data.to_dict()
                task_graph.append(task_graph_item)

        uploaded_by: None | str | Unset
        if isinstance(self.uploaded_by, Unset):
            uploaded_by = UNSET
        else:
            uploaded_by = self.uploaded_by

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
        if describe_error is not UNSET:
            field_dict["describe_error"] = describe_error
        if description is not UNSET:
            field_dict["description"] = description
        if needs_new_build is not UNSET:
            field_dict["needs_new_build"] = needs_new_build
        if not_loaded_reason is not UNSET:
            field_dict["not_loaded_reason"] = not_loaded_reason
        if paused is not UNSET:
            field_dict["paused"] = paused
        if plugin_interface_version is not UNSET:
            field_dict["plugin_interface_version"] = plugin_interface_version
        if priority is not UNSET:
            field_dict["priority"] = priority
        if priority_notice is not UNSET:
            field_dict["priority_notice"] = priority_notice
        if task_graph is not UNSET:
            field_dict["task_graph"] = task_graph
        if uploaded_by is not UNSET:
            field_dict["uploaded_by"] = uploaded_by

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.input_slot import InputSlot
        from ..models.needs_new_build import NeedsNewBuild
        from ..models.priority_grant import PriorityGrant
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

        def _parse_describe_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        describe_error = _parse_describe_error(d.pop("describe_error", UNSET))

        def _parse_description(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_needs_new_build(data: object) -> NeedsNewBuild | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                needs_new_build_type_0 = NeedsNewBuild.from_dict(data)

                return needs_new_build_type_0
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(NeedsNewBuild | None | Unset, data)

        needs_new_build = _parse_needs_new_build(d.pop("needs_new_build", UNSET))

        def _parse_not_loaded_reason(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        not_loaded_reason = _parse_not_loaded_reason(d.pop("not_loaded_reason", UNSET))

        paused = d.pop("paused", UNSET)

        def _parse_plugin_interface_version(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        plugin_interface_version = _parse_plugin_interface_version(
            d.pop("plugin_interface_version", UNSET)
        )

        def _parse_priority(data: object) -> None | PriorityGrant | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                priority_type_0 = PriorityGrant.from_dict(data)

                return priority_type_0
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(None | PriorityGrant | Unset, data)

        priority = _parse_priority(d.pop("priority", UNSET))

        def _parse_priority_notice(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        priority_notice = _parse_priority_notice(d.pop("priority_notice", UNSET))

        _task_graph = d.pop("task_graph", UNSET)
        task_graph: list[WorkflowTaskNode] | Unset = UNSET
        if _task_graph is not UNSET:
            task_graph = []
            for task_graph_item_data in _task_graph:
                task_graph_item = WorkflowTaskNode.from_dict(task_graph_item_data)

                task_graph.append(task_graph_item)

        def _parse_uploaded_by(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        uploaded_by = _parse_uploaded_by(d.pop("uploaded_by", UNSET))

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
            describe_error=describe_error,
            description=description,
            needs_new_build=needs_new_build,
            not_loaded_reason=not_loaded_reason,
            paused=paused,
            plugin_interface_version=plugin_interface_version,
            priority=priority,
            priority_notice=priority_notice,
            task_graph=task_graph,
            uploaded_by=uploaded_by,
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
