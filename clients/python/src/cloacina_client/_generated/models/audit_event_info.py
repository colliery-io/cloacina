from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.audit_event_info_detail import AuditEventInfoDetail


T = TypeVar("T", bound="AuditEventInfo")


@_attrs_define
class AuditEventInfo:
    """One audit row: who did what, when, in which tenant.

    Attributes:
        action (str): A fixed string, for example `package.upload`.
        actor_key_id (str): UUID of the API key that made the request.
        actor_key_name (str): Name of that key.
        created_at (str): RFC 3339 timestamp.
        detail (AuditEventInfoDetail): A small JSON object with facts of the action (a version, an
            execution id). Never a secret value or a request body.
        id (int): Row id. Larger is newer.
        outcome (str): `ok` or `denied`.
        target_kind (str): What the action applied to, for example `package` or `workflow`.
        actor_identity (None | str | Unset): Who the key was minted for at login (`oidc:<issuer>:<sub>` or
            `local:<account_id>`); `null` for a manually created key.
        target (None | str | Unset): Name or id of the target; `null` when the action has none.
        tenant_id (None | str | Unset): The tenant the action applied to; `null` for a platform action.
    """

    action: str
    actor_key_id: str
    actor_key_name: str
    created_at: str
    detail: AuditEventInfoDetail
    id: int
    outcome: str
    target_kind: str
    actor_identity: None | str | Unset = UNSET
    target: None | str | Unset = UNSET
    tenant_id: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        action = self.action

        actor_key_id = self.actor_key_id

        actor_key_name = self.actor_key_name

        created_at = self.created_at

        detail = self.detail.to_dict()

        id = self.id

        outcome = self.outcome

        target_kind = self.target_kind

        actor_identity: None | str | Unset
        if isinstance(self.actor_identity, Unset):
            actor_identity = UNSET
        else:
            actor_identity = self.actor_identity

        target: None | str | Unset
        if isinstance(self.target, Unset):
            target = UNSET
        else:
            target = self.target

        tenant_id: None | str | Unset
        if isinstance(self.tenant_id, Unset):
            tenant_id = UNSET
        else:
            tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "action": action,
                "actor_key_id": actor_key_id,
                "actor_key_name": actor_key_name,
                "created_at": created_at,
                "detail": detail,
                "id": id,
                "outcome": outcome,
                "target_kind": target_kind,
            }
        )
        if actor_identity is not UNSET:
            field_dict["actor_identity"] = actor_identity
        if target is not UNSET:
            field_dict["target"] = target
        if tenant_id is not UNSET:
            field_dict["tenant_id"] = tenant_id

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.audit_event_info_detail import AuditEventInfoDetail

        d = dict(src_dict)
        action = d.pop("action")

        actor_key_id = d.pop("actor_key_id")

        actor_key_name = d.pop("actor_key_name")

        created_at = d.pop("created_at")

        detail = AuditEventInfoDetail.from_dict(d.pop("detail"))

        id = d.pop("id")

        outcome = d.pop("outcome")

        target_kind = d.pop("target_kind")

        def _parse_actor_identity(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        actor_identity = _parse_actor_identity(d.pop("actor_identity", UNSET))

        def _parse_target(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        target = _parse_target(d.pop("target", UNSET))

        def _parse_tenant_id(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        tenant_id = _parse_tenant_id(d.pop("tenant_id", UNSET))

        audit_event_info = cls(
            action=action,
            actor_key_id=actor_key_id,
            actor_key_name=actor_key_name,
            created_at=created_at,
            detail=detail,
            id=id,
            outcome=outcome,
            target_kind=target_kind,
            actor_identity=actor_identity,
            target=target,
            tenant_id=tenant_id,
        )

        audit_event_info.additional_properties = d
        return audit_event_info

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
