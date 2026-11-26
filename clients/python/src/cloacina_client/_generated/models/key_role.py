from enum import StrEnum


class KeyRole(StrEnum):
    ADMIN = "admin"
    AGENT = "agent"
    READ = "read"
    WRITE = "write"

    def __str__(self) -> str:
        return str(self.value)
