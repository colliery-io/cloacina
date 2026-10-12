from enum import StrEnum


class FireMode(StrEnum):
    FIRE_WITH = "fire_with"
    FORCE_FIRE = "force_fire"

    def __str__(self) -> str:
        return str(self.value)
