from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.sync_pass import SyncPass
    from ..models.sync_side import SyncSide


T = TypeVar("T", bound="SyncDetail")


@_attrs_define
class SyncDetail:
    """A sync and its params, as stored: the params name secrets, never hold them.

    Attributes:
        dead_letters (int):
        destination (SyncSide): One side of a sync: a compiled connector, or a connector definition.
        enabled (bool):
        name (str): The sync definition's name: the workflow instance's.
        schema_broken (bool): Whether a stream's schema is broken.
        source (SyncSide): One side of a sync: a compiled connector, or a connector definition.
        stream (str):
        sync_mode (str): `FullRefresh`, `Incremental` or `Cdc`.
        sync_name (str): The name its cursor and dead-letters are kept under.
        write_mode (str): `Append`, `Upsert` or `Overwrite`.
        cron (None | str | Unset):
        last_pass (None | SyncPass | Unset):
        next_run_at (None | str | Unset):
        timezone (None | str | Unset):
        params (Any | Unset):
    """

    dead_letters: int
    destination: SyncSide
    enabled: bool
    name: str
    schema_broken: bool
    source: SyncSide
    stream: str
    sync_mode: str
    sync_name: str
    write_mode: str
    cron: None | str | Unset = UNSET
    last_pass: None | SyncPass | Unset = UNSET
    next_run_at: None | str | Unset = UNSET
    timezone: None | str | Unset = UNSET
    params: Any | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.sync_pass import SyncPass

        dead_letters = self.dead_letters

        destination = self.destination.to_dict()

        enabled = self.enabled

        name = self.name

        schema_broken = self.schema_broken

        source = self.source.to_dict()

        stream = self.stream

        sync_mode = self.sync_mode

        sync_name = self.sync_name

        write_mode = self.write_mode

        cron: None | str | Unset
        if isinstance(self.cron, Unset):
            cron = UNSET
        else:
            cron = self.cron

        last_pass: dict[str, Any] | None | Unset
        if isinstance(self.last_pass, Unset):
            last_pass = UNSET
        elif isinstance(self.last_pass, SyncPass):
            last_pass = self.last_pass.to_dict()
        else:
            last_pass = self.last_pass

        next_run_at: None | str | Unset
        if isinstance(self.next_run_at, Unset):
            next_run_at = UNSET
        else:
            next_run_at = self.next_run_at

        timezone: None | str | Unset
        if isinstance(self.timezone, Unset):
            timezone = UNSET
        else:
            timezone = self.timezone

        params = self.params

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "dead_letters": dead_letters,
                "destination": destination,
                "enabled": enabled,
                "name": name,
                "schema_broken": schema_broken,
                "source": source,
                "stream": stream,
                "sync_mode": sync_mode,
                "sync_name": sync_name,
                "write_mode": write_mode,
            }
        )
        if cron is not UNSET:
            field_dict["cron"] = cron
        if last_pass is not UNSET:
            field_dict["last_pass"] = last_pass
        if next_run_at is not UNSET:
            field_dict["next_run_at"] = next_run_at
        if timezone is not UNSET:
            field_dict["timezone"] = timezone
        if params is not UNSET:
            field_dict["params"] = params

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.sync_pass import SyncPass
        from ..models.sync_side import SyncSide

        d = dict(src_dict)
        dead_letters = d.pop("dead_letters")

        destination = SyncSide.from_dict(d.pop("destination"))

        enabled = d.pop("enabled")

        name = d.pop("name")

        schema_broken = d.pop("schema_broken")

        source = SyncSide.from_dict(d.pop("source"))

        stream = d.pop("stream")

        sync_mode = d.pop("sync_mode")

        sync_name = d.pop("sync_name")

        write_mode = d.pop("write_mode")

        def _parse_cron(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        cron = _parse_cron(d.pop("cron", UNSET))

        def _parse_last_pass(data: object) -> None | SyncPass | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                last_pass_type_0 = SyncPass.from_dict(data)

                return last_pass_type_0
            except (TypeError, ValueError, AttributeError, KeyError):
                pass
            return cast(None | SyncPass | Unset, data)

        last_pass = _parse_last_pass(d.pop("last_pass", UNSET))

        def _parse_next_run_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        next_run_at = _parse_next_run_at(d.pop("next_run_at", UNSET))

        def _parse_timezone(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        timezone = _parse_timezone(d.pop("timezone", UNSET))

        params = d.pop("params", UNSET)

        sync_detail = cls(
            dead_letters=dead_letters,
            destination=destination,
            enabled=enabled,
            name=name,
            schema_broken=schema_broken,
            source=source,
            stream=stream,
            sync_mode=sync_mode,
            sync_name=sync_name,
            write_mode=write_mode,
            cron=cron,
            last_pass=last_pass,
            next_run_at=next_run_at,
            timezone=timezone,
            params=params,
        )

        sync_detail.additional_properties = d
        return sync_detail

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
