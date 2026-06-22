from enum import Enum


class FireMode(str, Enum):
    FIRE_WITH = "fire_with"
    FORCE_FIRE = "force_fire"

    def __str__(self) -> str:
        return str(self.value)
