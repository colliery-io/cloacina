from __future__ import annotations

from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Self, TypeVar, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.agent_egress_rule_info import AgentEgressRuleInfo
    from ..models.agent_pool_settings_info_pod_annotations import (
        AgentPoolSettingsInfoPodAnnotations,
    )
    from ..models.agent_pool_settings_info_pod_labels import (
        AgentPoolSettingsInfoPodLabels,
    )
    from ..models.agent_pool_settings_info_service_account_annotations import (
        AgentPoolSettingsInfoServiceAccountAnnotations,
    )


T = TypeVar("T", bound="AgentPoolSettingsInfo")


@_attrs_define
class AgentPoolSettingsInfo:
    """The settings of one agent pool, as the pool routes return them. The same
    fields and the same JSON as [`AgentPoolSettings`], but an unknown field is
    ignored: a client reads an answer of a newer server (CLOACINA-T-1149).

        Attributes:
            automount_service_account_token (bool | None | Unset): The pod's `automountServiceAccountToken`. Absent means
                `false`.
            cpu_limit (None | str | Unset): CPU limit of the agent container, a Kubernetes quantity (`4`).
            cpu_request (None | str | Unset): CPU request of the agent container, a Kubernetes quantity (`500m`).
            extra_egress (list[AgentEgressRuleInfo] | Unset): Extra egress rules for the pool's pods, added to the default
                (cluster
                DNS and the server only).
            image (None | str | Unset): Agent image. Absent: the server-wide `CLOACINA_AGENT_IMAGE`.
            max_agents (int | None | Unset): Most agents of the pool.
            max_concurrency (int | None | Unset): How many tasks one agent runs at the same time. Absent: the agent's
                own default.
            memory_limit (None | str | Unset): Memory limit of the agent container, a Kubernetes quantity (`32Gi`).
            memory_request (None | str | Unset): Memory request of the agent container, a Kubernetes quantity (`8Gi`).
            min_agents (int | None | Unset): Fewest agents of the pool. Absent: the pool keeps `max_agents` agents.
            pod_annotations (AgentPoolSettingsInfoPodAnnotations | Unset): Annotations added to the pool's pods.
            pod_labels (AgentPoolSettingsInfoPodLabels | Unset): Labels added to the pool's pods.
            service_account_annotations (AgentPoolSettingsInfoServiceAccountAnnotations | Unset): Annotations the actuator
                puts on that ServiceAccount.
            service_account_name (None | str | Unset): Name of the Kubernetes ServiceAccount the agent pods run as. Absent:
                the namespace's `default` ServiceAccount.
    """

    automount_service_account_token: bool | None | Unset = UNSET
    cpu_limit: None | str | Unset = UNSET
    cpu_request: None | str | Unset = UNSET
    extra_egress: list[AgentEgressRuleInfo] | Unset = UNSET
    image: None | str | Unset = UNSET
    max_agents: int | None | Unset = UNSET
    max_concurrency: int | None | Unset = UNSET
    memory_limit: None | str | Unset = UNSET
    memory_request: None | str | Unset = UNSET
    min_agents: int | None | Unset = UNSET
    pod_annotations: AgentPoolSettingsInfoPodAnnotations | Unset = UNSET
    pod_labels: AgentPoolSettingsInfoPodLabels | Unset = UNSET
    service_account_annotations: (
        AgentPoolSettingsInfoServiceAccountAnnotations | Unset
    ) = UNSET
    service_account_name: None | str | Unset = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

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
        field_dict.update(self.additional_properties)
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
        from ..models.agent_egress_rule_info import AgentEgressRuleInfo
        from ..models.agent_pool_settings_info_pod_annotations import (
            AgentPoolSettingsInfoPodAnnotations,
        )
        from ..models.agent_pool_settings_info_pod_labels import (
            AgentPoolSettingsInfoPodLabels,
        )
        from ..models.agent_pool_settings_info_service_account_annotations import (
            AgentPoolSettingsInfoServiceAccountAnnotations,
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
        extra_egress: list[AgentEgressRuleInfo] | Unset = UNSET
        if _extra_egress is not UNSET:
            extra_egress = []
            for extra_egress_item_data in _extra_egress:
                extra_egress_item = AgentEgressRuleInfo.from_dict(
                    extra_egress_item_data
                )

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
        pod_annotations: AgentPoolSettingsInfoPodAnnotations | Unset
        if isinstance(_pod_annotations, Unset):
            pod_annotations = UNSET
        else:
            pod_annotations = AgentPoolSettingsInfoPodAnnotations.from_dict(
                _pod_annotations
            )

        _pod_labels = d.pop("pod_labels", UNSET)
        pod_labels: AgentPoolSettingsInfoPodLabels | Unset
        if isinstance(_pod_labels, Unset):
            pod_labels = UNSET
        else:
            pod_labels = AgentPoolSettingsInfoPodLabels.from_dict(_pod_labels)

        _service_account_annotations = d.pop("service_account_annotations", UNSET)
        service_account_annotations: (
            AgentPoolSettingsInfoServiceAccountAnnotations | Unset
        )
        if isinstance(_service_account_annotations, Unset):
            service_account_annotations = UNSET
        else:
            service_account_annotations = (
                AgentPoolSettingsInfoServiceAccountAnnotations.from_dict(
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

        agent_pool_settings_info = cls(
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

        agent_pool_settings_info.additional_properties = d
        return agent_pool_settings_info

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
