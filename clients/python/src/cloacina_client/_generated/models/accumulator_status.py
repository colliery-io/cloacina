from __future__ import annotations

from collections.abc import Mapping
from typing import Any, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="AccumulatorStatus")


@_attrs_define
class AccumulatorStatus:
    """One row in `GET /v1/health/accumulators`.

    Attributes:
        name (str):
        status (Any): Accumulator health as reported by the endpoint registry. Free-form
            JSON for now; structured in a later contract revision.
        error (None | str | Unset): Degradation detail when the source is unhealthy (e.g. connection error).
        events_total (int | None | Unset): Total boundaries emitted since load (monotonic). `None` when untracked.
        last_event_at (None | str | Unset): Wall-clock of the last boundary this accumulator emitted (RFC3339), or
            `None` if it hasn't emitted yet / the runtime predates freshness tracking.
        last_operator_inject_at (None | str | Unset): RFC 3339 time of the last operator inject, or `None` if never
            (CLOACI-T-0776).
        operator_injects (int | Unset): Number of operator injects into this accumulator since load
            (CLOACI-T-0776) — manual interventions via the REST/WS inject paths. `0`
            when never injected. The UI marks injected accumulators with a "manual" pill.
        reactor (None | str | Unset): The reactor (graph) this accumulator feeds, self-registered by the graph
            at load (CLOACI-I-0128 follow-up). `None` for older runtimes that didn't
            register the descriptor. Lets an operator see what pushing to
            `/v1/ws/accumulator/{name}` actually drives.
        state (None | str | Unset): Health state label (`live`/`socket_only`/`disconnected`/…), CLOACI-T-0765.
            Mirrors the `state` inside `status`; promoted to a typed field for the UI.
        tenant_id (None | str | Unset): Owning tenant, or `None` for untagged single-tenant graphs.
    """

    name: str
    status: Any
    error: None | str | Unset = UNSET
    events_total: int | None | Unset = UNSET
    last_event_at: None | str | Unset = UNSET
    last_operator_inject_at: None | str | Unset = UNSET
    operator_injects: int | Unset = UNSET
    reactor: None | str | Unset = UNSET
    state: None | str | Unset = UNSET
    tenant_id: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        name = self.name

        status = self.status

        error: None | str | Unset
        if isinstance(self.error, Unset):
            error = UNSET
        else:
            error = self.error

        events_total: int | None | Unset
        if isinstance(self.events_total, Unset):
            events_total = UNSET
        else:
            events_total = self.events_total

        last_event_at: None | str | Unset
        if isinstance(self.last_event_at, Unset):
            last_event_at = UNSET
        else:
            last_event_at = self.last_event_at

        last_operator_inject_at: None | str | Unset
        if isinstance(self.last_operator_inject_at, Unset):
            last_operator_inject_at = UNSET
        else:
            last_operator_inject_at = self.last_operator_inject_at

        operator_injects = self.operator_injects

        reactor: None | str | Unset
        if isinstance(self.reactor, Unset):
            reactor = UNSET
        else:
            reactor = self.reactor

        state: None | str | Unset
        if isinstance(self.state, Unset):
            state = UNSET
        else:
            state = self.state

        tenant_id: None | str | Unset
        if isinstance(self.tenant_id, Unset):
            tenant_id = UNSET
        else:
            tenant_id = self.tenant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "name": name,
                "status": status,
            }
        )
        if error is not UNSET:
            field_dict["error"] = error
        if events_total is not UNSET:
            field_dict["events_total"] = events_total
        if last_event_at is not UNSET:
            field_dict["last_event_at"] = last_event_at
        if last_operator_inject_at is not UNSET:
            field_dict["last_operator_inject_at"] = last_operator_inject_at
        if operator_injects is not UNSET:
            field_dict["operator_injects"] = operator_injects
        if reactor is not UNSET:
            field_dict["reactor"] = reactor
        if state is not UNSET:
            field_dict["state"] = state
        if tenant_id is not UNSET:
            field_dict["tenant_id"] = tenant_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        name = d.pop("name")

        status = d.pop("status")

        def _parse_error(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        error = _parse_error(d.pop("error", UNSET))

        def _parse_events_total(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        events_total = _parse_events_total(d.pop("events_total", UNSET))

        def _parse_last_event_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        last_event_at = _parse_last_event_at(d.pop("last_event_at", UNSET))

        def _parse_last_operator_inject_at(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        last_operator_inject_at = _parse_last_operator_inject_at(
            d.pop("last_operator_inject_at", UNSET)
        )

        operator_injects = d.pop("operator_injects", UNSET)

        def _parse_reactor(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        reactor = _parse_reactor(d.pop("reactor", UNSET))

        def _parse_state(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        state = _parse_state(d.pop("state", UNSET))

        def _parse_tenant_id(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        tenant_id = _parse_tenant_id(d.pop("tenant_id", UNSET))

        accumulator_status = cls(
            name=name,
            status=status,
            error=error,
            events_total=events_total,
            last_event_at=last_event_at,
            last_operator_inject_at=last_operator_inject_at,
            operator_injects=operator_injects,
            reactor=reactor,
            state=state,
            tenant_id=tenant_id,
        )

        accumulator_status.additional_properties = d
        return accumulator_status

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
