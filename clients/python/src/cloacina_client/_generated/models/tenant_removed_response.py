from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="TenantRemovedResponse")


@_attrs_define
class TenantRemovedResponse:
    """`DELETE /tenants/{schema_name}` response — orchestrated teardown report.

    Attributes:
        db_cache_evicted (bool):
        revoked_keys (int): Number of still-active API keys revoked during teardown.
        runner_evicted (bool):
        schema_name (str):
        status (str): Always `"removed"`.
    """

    db_cache_evicted: bool
    revoked_keys: int
    runner_evicted: bool
    schema_name: str
    status: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        db_cache_evicted = self.db_cache_evicted

        revoked_keys = self.revoked_keys

        runner_evicted = self.runner_evicted

        schema_name = self.schema_name

        status = self.status

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "db_cache_evicted": db_cache_evicted,
                "revoked_keys": revoked_keys,
                "runner_evicted": runner_evicted,
                "schema_name": schema_name,
                "status": status,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        db_cache_evicted = d.pop("db_cache_evicted")

        revoked_keys = d.pop("revoked_keys")

        runner_evicted = d.pop("runner_evicted")

        schema_name = d.pop("schema_name")

        status = d.pop("status")

        tenant_removed_response = cls(
            db_cache_evicted=db_cache_evicted,
            revoked_keys=revoked_keys,
            runner_evicted=runner_evicted,
            schema_name=schema_name,
            status=status,
        )

        tenant_removed_response.additional_properties = d
        return tenant_removed_response

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
