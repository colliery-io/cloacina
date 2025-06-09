"""
Utility functions for Cloaca tasks.
"""

import re
import shutil
from pathlib import Path

import angreal  # type: ignore


class FileOperationError(Exception):
    """Raised when file operations fail."""
    pass


def write_file_safe(path: Path, content: str, encoding: str = "utf-8", backup: bool = False):
    """Safely write a file with error handling.

    Args:
        path: File path to write
        content: Content to write
        encoding: File encoding
        backup: Whether to backup existing file

    Returns:
        Path to backup file if backup=True and file existed, None otherwise

    Raises:
        FileOperationError: If file cannot be written
    """
    try:
        backup_path = None

        if backup and path.exists():
            backup_path = path.with_suffix(path.suffix + ".backup")
            shutil.copy2(path, backup_path)

        # Ensure parent directory exists
        path.parent.mkdir(parents=True, exist_ok=True)

        path.write_text(content, encoding=encoding)
        return backup_path

    except (OSError, UnicodeEncodeError) as e:
        raise FileOperationError(f"Failed to write file {path}: {e}")


def get_workspace_version() -> str:
    """Extract version from workspace Cargo.toml.

    Returns:
        Version string from workspace configuration

    Raises:
        ValueError: If version cannot be found in workspace Cargo.toml
    """
    project_root = Path(angreal.get_root()).parent
    cargo_toml = project_root / "Cargo.toml"

    if not cargo_toml.exists():
        raise FileNotFoundError(f"Workspace Cargo.toml not found at {cargo_toml}")

    content = cargo_toml.read_text()
    match = re.search(r'\[workspace\.package\].*?version\s*=\s*"([^"]+)"', content, re.DOTALL)

    if match:
        return match.group(1)

    raise ValueError("Could not find version in workspace Cargo.toml")
