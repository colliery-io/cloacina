from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="WorkflowSourceFile")


@_attrs_define
class WorkflowSourceFile:
    """One source file from a workflow package's retained `.cloacina` archive,
    surfaced read-only for display (CLOACI-T-0750).

        Attributes:
            contents (str): UTF-8 file contents.
            path (str): Path relative to the package source root, using forward slashes
                (e.g. `"src/lib.rs"`, `"package.toml"`).
            language (None | str | Unset): Best-effort language id derived from the file extension (`"rust"`,
                `"python"`, `"toml"`, …), for syntax highlighting. `None` when unknown.
    """

    contents: str
    path: str
    language: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        contents = self.contents

        path = self.path

        language: None | str | Unset
        if isinstance(self.language, Unset):
            language = UNSET
        else:
            language = self.language

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "contents": contents,
                "path": path,
            }
        )
        if language is not UNSET:
            field_dict["language"] = language

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        contents = d.pop("contents")

        path = d.pop("path")

        def _parse_language(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        language = _parse_language(d.pop("language", UNSET))

        workflow_source_file = cls(
            contents=contents,
            path=path,
            language=language,
        )

        workflow_source_file.additional_properties = d
        return workflow_source_file

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
