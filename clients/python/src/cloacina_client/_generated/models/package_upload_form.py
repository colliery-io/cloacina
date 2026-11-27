from __future__ import annotations

from collections.abc import Mapping
from io import BytesIO
from typing import Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from .. import types
from ..types import UNSET, File, Unset

T = TypeVar("T", bound="PackageUploadForm")


@_attrs_define
class PackageUploadForm:
    """Multipart form for workflow package upload. Spec-only type: the handler
    reads the fields `file` and `signature` by name.

        Attributes:
            file (File): The `.cloacina` package archive.
            signature (str | Unset): The detached signature of the archive: the JSON text of the `.sig`
                file that `cloacinactl package pack --sign` writes (`version`,
                `algorithm`, `package_hash`, `key_fingerprint`, `signature`,
                `signed_at`). Ed25519 over the SHA-256 of the archive. Needed when the
                server requires signed packages; checked whenever it is sent.
                Left out of an upload with no signature: the field has no null value.
    """

    file: File
    signature: str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        file = self.file.to_tuple()

        signature = self.signature

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "file": file,
            }
        )
        if signature is not UNSET:
            field_dict["signature"] = signature

        return field_dict

    def to_multipart(self) -> types.RequestFiles:
        files: types.RequestFiles = []

        files.append(("file", self.file.to_tuple()))

        if not isinstance(self.signature, Unset):
            files.append(
                ("signature", (None, str(self.signature).encode(), "text/plain"))
            )

        for prop_name, prop in self.additional_properties.items():
            files.append((prop_name, (None, str(prop).encode(), "text/plain")))

        return files

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        file = File(payload=BytesIO(d.pop("file")))

        signature = d.pop("signature", UNSET)

        package_upload_form = cls(
            file=file,
            signature=signature,
        )

        package_upload_form.additional_properties = d
        return package_upload_form

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
