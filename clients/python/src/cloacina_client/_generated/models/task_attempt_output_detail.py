from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Self, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="TaskAttemptOutputDetail")


@_attrs_define
class TaskAttemptOutputDetail:
    """The end of what one attempt of a task wrote on stdout and stderr, as one
    stream (CLOACINA-T-1091). Kept for tasks that ran in an agent's worker
    process.

        Attributes:
            attempt (int): The attempt, from 1.
            cut (bool): `true` when `output` is not the whole stream: its start was cut.
            output (str): The last bytes of the output, at most the server's
                `--task-output-max-bytes` (256 KiB by default). The exact values of
                the attempt's own secrets are `***` when the agent masks them.
            recorded_at (str): RFC 3339 timestamp of when the output was stored.
            total_bytes (int): How many bytes the attempt wrote in all.
    """

    attempt: int
    cut: bool
    output: str
    recorded_at: str
    total_bytes: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        attempt = self.attempt

        cut = self.cut

        output = self.output

        recorded_at = self.recorded_at

        total_bytes = self.total_bytes

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "attempt": attempt,
                "cut": cut,
                "output": output,
                "recorded_at": recorded_at,
                "total_bytes": total_bytes,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls, src_dict: Mapping[str, Any]) -> Self:
        d = dict(src_dict)
        attempt = d.pop("attempt")

        cut = d.pop("cut")

        output = d.pop("output")

        recorded_at = d.pop("recorded_at")

        total_bytes = d.pop("total_bytes")

        task_attempt_output_detail = cls(
            attempt=attempt,
            cut=cut,
            output=output,
            recorded_at=recorded_at,
            total_bytes=total_bytes,
        )

        task_attempt_output_detail.additional_properties = d
        return task_attempt_output_detail

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
