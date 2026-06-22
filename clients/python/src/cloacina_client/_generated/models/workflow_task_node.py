from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="WorkflowTaskNode")


@_attrs_define
class WorkflowTaskNode:
    """One node in a workflow's task dependency graph — a task plus the ids of the
    tasks it depends on. The UI renders these as a DAG. (CLOACI-T-0663)

        Attributes:
            dependencies (list[str]): Local ids of the tasks this task depends on (its incoming edges).
            id (str): Local task id (the node id), e.g. `"validate"`.
            description (None | str | Unset): Optional human-readable task description.
            doc_what (None | str | Unset): CLOACI-T-0752 "what" — short summary parsed from the task's
                doc-comment/docstring at build time. `None` when undocumented.
            doc_why (None | str | Unset): CLOACI-T-0752 "why" — rationale parsed from the doc-comment/docstring.
                `None` when undocumented.
    """

    dependencies: list[str]
    id: str
    description: None | str | Unset = UNSET
    doc_what: None | str | Unset = UNSET
    doc_why: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        dependencies = self.dependencies

        id = self.id

        description: None | str | Unset
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        doc_what: None | str | Unset
        if isinstance(self.doc_what, Unset):
            doc_what = UNSET
        else:
            doc_what = self.doc_what

        doc_why: None | str | Unset
        if isinstance(self.doc_why, Unset):
            doc_why = UNSET
        else:
            doc_why = self.doc_why

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "dependencies": dependencies,
                "id": id,
            }
        )
        if description is not UNSET:
            field_dict["description"] = description
        if doc_what is not UNSET:
            field_dict["doc_what"] = doc_what
        if doc_why is not UNSET:
            field_dict["doc_why"] = doc_why

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        dependencies = cast(list[str], d.pop("dependencies"))

        id = d.pop("id")

        def _parse_description(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_doc_what(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        doc_what = _parse_doc_what(d.pop("doc_what", UNSET))

        def _parse_doc_why(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        doc_why = _parse_doc_why(d.pop("doc_why", UNSET))

        workflow_task_node = cls(
            dependencies=dependencies,
            id=id,
            description=description,
            doc_what=doc_what,
            doc_why=doc_why,
        )

        workflow_task_node.additional_properties = d
        return workflow_task_node

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
