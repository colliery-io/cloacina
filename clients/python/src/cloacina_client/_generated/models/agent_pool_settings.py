from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.agent_egress_rule import AgentEgressRule
    from ..models.agent_pool_settings_pod_annotations import (
        AgentPoolSettingsPodAnnotations,
    )
    from ..models.agent_pool_settings_pod_labels import AgentPoolSettingsPodLabels
    from ..models.agent_pool_settings_service_account_annotations import (
        AgentPoolSettingsServiceAccountAnnotations,
    )


T = TypeVar("T", bound="AgentPoolSettings")


@_attrs_define
class AgentPoolSettings:
    """Settings of one agent pool of a tenant (CLOACINA-T-1021).

    Each field is optional. A field that is absent means the server-wide
    default (`CLOACINA_AGENT_*` on the server). The settings apply to every
    managed agent of the pool: a pool has one size of agent.

        Attributes:
            automount_service_account_token (bool | None | Unset): The pod's `automountServiceAccountToken`. Absent means
                `false`: the
                pod mounts no ServiceAccount token. The agent does not call the
                Kubernetes API. Set `true` for a pool whose identity system needs it.
            cpu_limit (None | str | Unset): CPU limit of the agent container, a Kubernetes quantity (`4`).
            cpu_request (None | str | Unset): CPU request of the agent container, a Kubernetes quantity (`500m`).
            extra_egress (list[AgentEgressRule] | Unset): Extra egress rules for the pool's pods, added to the default
                (cluster
                DNS and the server only).
            image (None | str | Unset): Agent image. Absent: the server-wide `CLOACINA_AGENT_IMAGE`.
            max_agents (int | None | Unset): Most agents of the pool. Not for the pool `default` (its max is the
                tenant's agent limit, `/v1/tenants/{tenant_id}/limits`). Required for
                another pool.
            max_concurrency (int | None | Unset): How many tasks one agent runs at the same time (the agent's
                `--max-concurrency`). Absent: the agent's own default.
            memory_limit (None | str | Unset): Memory limit of the agent container, a Kubernetes quantity (`32Gi`).
            memory_request (None | str | Unset): Memory request of the agent container, a Kubernetes quantity (`8Gi`).
            min_agents (int | None | Unset): Fewest agents of the pool, at most `max_agents`. The pool scales
                between the two on the work that waits for it, and goes back to this
                count when it is idle; `0` lets it scale to zero. Absent: the pool
                keeps `max_agents` agents. Not for the pool `default` (its floor is the
                server-wide `CLOACINA_AUTOSCALE_FLOOR`).
            pod_annotations (AgentPoolSettingsPodAnnotations | Unset): Annotations added to the pool's pods (the pod
                template of its
                Deployment).
            pod_labels (AgentPoolSettingsPodLabels | Unset): Labels added to the pool's pods (the pod template of its
                Deployment).
                Some identity systems select pods by a label. The labels the actuator
                sets itself are refused.
            service_account_annotations (AgentPoolSettingsServiceAccountAnnotations | Unset): Annotations the actuator puts
                on that ServiceAccount (for a cloud
                workload identity). Needs `service_account_name`.
            service_account_name (None | str | Unset): Name of the Kubernetes ServiceAccount the agent pods run as. The
                actuator creates it in the tenant namespace. Absent: the namespace's
                `default` ServiceAccount.
    """

    automount_service_account_token: bool | None | Unset = UNSET
    cpu_limit: None | str | Unset = UNSET
    cpu_request: None | str | Unset = UNSET
    extra_egress: list[AgentEgressRule] | Unset = UNSET
    image: None | str | Unset = UNSET
    max_agents: int | None | Unset = UNSET
    max_concurrency: int | None | Unset = UNSET
    memory_limit: None | str | Unset = UNSET
    memory_request: None | str | Unset = UNSET
    min_agents: int | None | Unset = UNSET
    pod_annotations: AgentPoolSettingsPodAnnotations | Unset = UNSET
    pod_labels: AgentPoolSettingsPodLabels | Unset = UNSET
    service_account_annotations: AgentPoolSettingsServiceAccountAnnotations | Unset = (
        UNSET
    )
    service_account_name: None | str | Unset = UNSET

    def to_dict(self) -> dict[str, Any]:
        automount_service_account_token: bool | None | Unset
        if isinstance(self.automount_service_account_token, Unset):
            automount_service_account_token = UNSET
        else:
            automount_service_account_token = self.automount_service_account_token

        cpu_limit: None | str | Unset
        if isinstance(self.cpu_limit, Unset):
            cpu_limit = UNSET
        else:
            cpu_limit = self.cpu_limit

        cpu_request: None | str | Unset
        if isinstance(self.cpu_request, Unset):
            cpu_request = UNSET
        else:
            cpu_request = self.cpu_request

        extra_egress: list[dict[str, Any]] | Unset = UNSET
        if not isinstance(self.extra_egress, Unset):
            extra_egress = []
            for extra_egress_item_data in self.extra_egress:
                extra_egress_item = extra_egress_item_data.to_dict()
                extra_egress.append(extra_egress_item)

        image: None | str | Unset
        if isinstance(self.image, Unset):
            image = UNSET
        else:
            image = self.image

        max_agents: int | None | Unset
        if isinstance(self.max_agents, Unset):
            max_agents = UNSET
        else:
            max_agents = self.max_agents

        max_concurrency: int | None | Unset
        if isinstance(self.max_concurrency, Unset):
            max_concurrency = UNSET
        else:
            max_concurrency = self.max_concurrency

        memory_limit: None | str | Unset
        if isinstance(self.memory_limit, Unset):
            memory_limit = UNSET
        else:
            memory_limit = self.memory_limit

        memory_request: None | str | Unset
        if isinstance(self.memory_request, Unset):
            memory_request = UNSET
        else:
            memory_request = self.memory_request

        min_agents: int | None | Unset
        if isinstance(self.min_agents, Unset):
            min_agents = UNSET
        else:
            min_agents = self.min_agents

        pod_annotations: dict[str, Any] | Unset = UNSET
        if not isinstance(self.pod_annotations, Unset):
            pod_annotations = self.pod_annotations.to_dict()

        pod_labels: dict[str, Any] | Unset = UNSET
        if not isinstance(self.pod_labels, Unset):
            pod_labels = self.pod_labels.to_dict()

        service_account_annotations: dict[str, Any] | Unset = UNSET
        if not isinstance(self.service_account_annotations, Unset):
            service_account_annotations = self.service_account_annotations.to_dict()

        service_account_name: None | str | Unset
        if isinstance(self.service_account_name, Unset):
            service_account_name = UNSET
        else:
            service_account_name = self.service_account_name

        field_dict: dict[str, Any] = {}

        field_dict.update({})
        if automount_service_account_token is not UNSET:
            field_dict["automount_service_account_token"] = (
                automount_service_account_token
            )
        if cpu_limit is not UNSET:
            field_dict["cpu_limit"] = cpu_limit
        if cpu_request is not UNSET:
            field_dict["cpu_request"] = cpu_request
        if extra_egress is not UNSET:
            field_dict["extra_egress"] = extra_egress
        if image is not UNSET:
            field_dict["image"] = image
        if max_agents is not UNSET:
            field_dict["max_agents"] = max_agents
        if max_concurrency is not UNSET:
            field_dict["max_concurrency"] = max_concurrency
        if memory_limit is not UNSET:
            field_dict["memory_limit"] = memory_limit
        if memory_request is not UNSET:
            field_dict["memory_request"] = memory_request
        if min_agents is not UNSET:
            field_dict["min_agents"] = min_agents
        if pod_annotations is not UNSET:
            field_dict["pod_annotations"] = pod_annotations
        if pod_labels is not UNSET:
            field_dict["pod_labels"] = pod_labels
        if service_account_annotations is not UNSET:
            field_dict["service_account_annotations"] = service_account_annotations
        if service_account_name is not UNSET:
            field_dict["service_account_name"] = service_account_name

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        from ..models.agent_egress_rule import AgentEgressRule
        from ..models.agent_pool_settings_pod_annotations import (
            AgentPoolSettingsPodAnnotations,
        )
        from ..models.agent_pool_settings_pod_labels import AgentPoolSettingsPodLabels
        from ..models.agent_pool_settings_service_account_annotations import (
            AgentPoolSettingsServiceAccountAnnotations,
        )

        d = dict(src_dict)

        def _parse_automount_service_account_token(data: object) -> bool | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(bool | None | Unset, data)

        automount_service_account_token = _parse_automount_service_account_token(
            d.pop("automount_service_account_token", UNSET)
        )

        def _parse_cpu_limit(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        cpu_limit = _parse_cpu_limit(d.pop("cpu_limit", UNSET))

        def _parse_cpu_request(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        cpu_request = _parse_cpu_request(d.pop("cpu_request", UNSET))

        _extra_egress = d.pop("extra_egress", UNSET)
        extra_egress: list[AgentEgressRule] | Unset = UNSET
        if _extra_egress is not UNSET:
            extra_egress = []
            for extra_egress_item_data in _extra_egress:
                extra_egress_item = AgentEgressRule.from_dict(extra_egress_item_data)

                extra_egress.append(extra_egress_item)

        def _parse_image(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        image = _parse_image(d.pop("image", UNSET))

        def _parse_max_agents(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        max_agents = _parse_max_agents(d.pop("max_agents", UNSET))

        def _parse_max_concurrency(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        max_concurrency = _parse_max_concurrency(d.pop("max_concurrency", UNSET))

        def _parse_memory_limit(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        memory_limit = _parse_memory_limit(d.pop("memory_limit", UNSET))

        def _parse_memory_request(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        memory_request = _parse_memory_request(d.pop("memory_request", UNSET))

        def _parse_min_agents(data: object) -> int | None | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(int | None | Unset, data)

        min_agents = _parse_min_agents(d.pop("min_agents", UNSET))

        _pod_annotations = d.pop("pod_annotations", UNSET)
        pod_annotations: AgentPoolSettingsPodAnnotations | Unset
        if isinstance(_pod_annotations, Unset):
            pod_annotations = UNSET
        else:
            pod_annotations = AgentPoolSettingsPodAnnotations.from_dict(
                _pod_annotations
            )

        _pod_labels = d.pop("pod_labels", UNSET)
        pod_labels: AgentPoolSettingsPodLabels | Unset
        if isinstance(_pod_labels, Unset):
            pod_labels = UNSET
        else:
            pod_labels = AgentPoolSettingsPodLabels.from_dict(_pod_labels)

        _service_account_annotations = d.pop("service_account_annotations", UNSET)
        service_account_annotations: AgentPoolSettingsServiceAccountAnnotations | Unset
        if isinstance(_service_account_annotations, Unset):
            service_account_annotations = UNSET
        else:
            service_account_annotations = (
                AgentPoolSettingsServiceAccountAnnotations.from_dict(
                    _service_account_annotations
                )
            )

        def _parse_service_account_name(data: object) -> None | str | Unset:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(None | str | Unset, data)

        service_account_name = _parse_service_account_name(
            d.pop("service_account_name", UNSET)
        )

        agent_pool_settings = cls(
            automount_service_account_token=automount_service_account_token,
            cpu_limit=cpu_limit,
            cpu_request=cpu_request,
            extra_egress=extra_egress,
            image=image,
            max_agents=max_agents,
            max_concurrency=max_concurrency,
            memory_limit=memory_limit,
            memory_request=memory_request,
            min_agents=min_agents,
            pod_annotations=pod_annotations,
            pod_labels=pod_labels,
            service_account_annotations=service_account_annotations,
            service_account_name=service_account_name,
        )

        return agent_pool_settings
