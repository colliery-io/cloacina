---
id: dependency-vendoring-with-uv-and
level: task
title: "Dependency vendoring with uv and platform wheel selection"
short_code: "CLOACI-T-0068"
created_at: 2026-01-28T14:29:02.749424+00:00
updated_at: 2026-01-28T14:29:02.749424+00:00
parent: CLOACI-I-0020
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0020
---

# Dependency vendoring with uv and platform wheel selection

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0020]]

## Objective

Implement dependency resolution and vendoring using `uv`. Download platform-specific wheels based on target architecture, extract them to `vendor/` directory, and generate `requirements.lock` for reproducibility.

## Acceptance Criteria

- [ ] `uv` CLI invocation resolves dependencies from pyproject.toml
- [ ] Platform-specific wheels downloaded for target architecture
- [ ] Wheels extracted to `vendor/` directory with correct structure
- [ ] `requirements.lock` generated with pinned versions and hashes
- [ ] Cross-platform builds work (build linux wheels on macos)
- [ ] Pure-python wheels handled correctly (no platform suffix)
- [ ] Source distributions (sdist) rejected with clear error

## Implementation Notes

### Platform Tag Mapping

Map our target identifiers to Python wheel platform tags:

```python
# cloaca/vendoring.py

PLATFORM_TO_WHEEL_TAGS = {
    "linux-x86_64": ["manylinux_2_17_x86_64", "manylinux2014_x86_64", "linux_x86_64"],
    "linux-arm64": ["manylinux_2_17_aarch64", "manylinux2014_aarch64", "linux_aarch64"],
    "macos-x86_64": ["macosx_10_9_x86_64", "macosx_10_12_x86_64", "macosx_11_0_x86_64"],
    "macos-arm64": ["macosx_11_0_arm64", "macosx_12_0_arm64"],
}

# Pure Python wheels (no platform-specific code)
PURE_PYTHON_TAGS = ["py3-none-any", "py2.py3-none-any"]
```

### uv CLI Integration

```python
# cloaca/vendoring.py
import subprocess
import tempfile
from pathlib import Path

def resolve_dependencies(
    pyproject_path: Path,
    target_platform: str,
    python_version: str = "3.11",
) -> list[ResolvedDependency]:
    """
    Use uv to resolve dependencies for target platform.

    uv pip compile produces a lock file with exact versions and hashes.
    """
    with tempfile.NamedTemporaryFile(suffix=".txt", delete=False) as lockfile:
        cmd = [
            "uv", "pip", "compile",
            str(pyproject_path),
            "--output-file", lockfile.name,
            "--python-version", python_version,
            "--platform", _uv_platform(target_platform),
            "--generate-hashes",
            "--no-emit-package", "cloaca",  # exclude the package itself
        ]

        result = subprocess.run(cmd, capture_output=True, text=True, check=False)
        if result.returncode != 0:
            raise DependencyResolutionError(f"uv failed: {result.stderr}")

        return _parse_requirements_txt(Path(lockfile.name))


def _uv_platform(target: str) -> str:
    """Convert our platform target to uv's --platform format."""
    mapping = {
        "linux-x86_64": "linux",
        "linux-arm64": "linux",
        "macos-x86_64": "macos",
        "macos-arm64": "macos",
    }
    return mapping[target]
```

### Wheel Download and Extraction

```python
# cloaca/vendoring.py
import zipfile
from dataclasses import dataclass

@dataclass
class ResolvedDependency:
    name: str
    version: str
    url: str | None  # Direct URL if available
    hash_sha256: str
    is_pure_python: bool


def download_wheels(
    dependencies: list[ResolvedDependency],
    target_platform: str,
    cache_dir: Path,
) -> list[Path]:
    """
    Download wheels for all dependencies.

    Uses uv pip download for efficient caching and parallel downloads.
    """
    wheel_dir = cache_dir / "wheels" / target_platform
    wheel_dir.mkdir(parents=True, exist_ok=True)

    # Write requirements to temp file
    with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
        for dep in dependencies:
            f.write(f"{dep.name}=={dep.version} --hash=sha256:{dep.hash_sha256}\n")
        req_file = Path(f.name)

    cmd = [
        "uv", "pip", "download",
        "-r", str(req_file),
        "--dest", str(wheel_dir),
        "--platform", _uv_platform(target_platform),
        "--python-version", "3.11",
        "--only-binary", ":all:",  # Reject sdists
    ]

    result = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        # Check if it's an sdist-only package
        if "No matching distribution" in result.stderr:
            raise SdistOnlyError(
                "Some packages only have source distributions. "
                "Python packages must have pre-built wheels for packaging."
            )
        raise DownloadError(f"uv download failed: {result.stderr}")

    return list(wheel_dir.glob("*.whl"))


def extract_wheels(wheels: list[Path], vendor_dir: Path) -> None:
    """
    Extract wheels to vendor directory.

    Wheel structure:
        package_name-1.0.0-py3-none-any.whl contains:
        - package_name/
        - package_name-1.0.0.dist-info/

    We extract to:
        vendor/
        - package_name/
        - package_name-1.0.0.dist-info/
    """
    vendor_dir.mkdir(parents=True, exist_ok=True)

    for wheel_path in wheels:
        with zipfile.ZipFile(wheel_path, 'r') as whl:
            # Extract all contents to vendor directory
            whl.extractall(vendor_dir)

        # Record what we extracted
        wheel_name = wheel_path.stem
        _record_vendored_wheel(vendor_dir, wheel_name)


def _record_vendored_wheel(vendor_dir: Path, wheel_name: str) -> None:
    """Record vendored wheel in VENDORED.txt for debugging."""
    record_file = vendor_dir / "VENDORED.txt"
    with record_file.open('a') as f:
        f.write(f"{wheel_name}\n")
```

### Lock File Generation

```python
# cloaca/vendoring.py

def generate_lock_file(
    dependencies: list[ResolvedDependency],
    output_path: Path,
    target_platform: str,
) -> None:
    """
    Generate requirements.lock with pinned versions and hashes.

    Format:
        # Generated by cloaca build for linux-x86_64
        # Python 3.11
        requests==2.31.0 \
            --hash=sha256:abc123...
        urllib3==2.0.4 \
            --hash=sha256:def456...
    """
    lines = [
        f"# Generated by cloaca build for {target_platform}",
        f"# Python 3.11",
        "#",
    ]

    for dep in sorted(dependencies, key=lambda d: d.name.lower()):
        lines.append(f"{dep.name}=={dep.version} \\")
        lines.append(f"    --hash=sha256:{dep.hash_sha256}")

    output_path.write_text("\n".join(lines) + "\n")
```

### Complete Vendoring Pipeline

```python
# cloaca/vendoring.py

def vendor_dependencies(
    pyproject_path: Path,
    target_platform: str,
    output_dir: Path,
    cache_dir: Path | None = None,
) -> VendorResult:
    """
    Complete vendoring pipeline for a target platform.

    1. Resolve dependencies with uv
    2. Download platform-specific wheels
    3. Extract to vendor/
    4. Generate requirements.lock
    """
    if cache_dir is None:
        cache_dir = Path.home() / ".cache" / "cloaca"

    vendor_dir = output_dir / "vendor"

    # Step 1: Resolve
    print(f"Resolving dependencies for {target_platform}...")
    dependencies = resolve_dependencies(pyproject_path, target_platform)

    if not dependencies:
        print("No dependencies to vendor")
        return VendorResult(vendor_dir=vendor_dir, dependencies=[], lock_file=None)

    # Step 2: Download
    print(f"Downloading {len(dependencies)} wheels...")
    wheels = download_wheels(dependencies, target_platform, cache_dir)

    # Step 3: Extract
    print(f"Extracting wheels to {vendor_dir}...")
    extract_wheels(wheels, vendor_dir)

    # Step 4: Generate lock file
    lock_file = output_dir / "requirements.lock"
    generate_lock_file(dependencies, lock_file, target_platform)
    print(f"Generated {lock_file}")

    return VendorResult(
        vendor_dir=vendor_dir,
        dependencies=dependencies,
        lock_file=lock_file,
    )


@dataclass
class VendorResult:
    vendor_dir: Path
    dependencies: list[ResolvedDependency]
    lock_file: Path | None
```

### Error Handling

```python
# cloaca/errors.py

class VendoringError(Exception):
    """Base class for vendoring errors."""
    pass

class DependencyResolutionError(VendoringError):
    """Failed to resolve dependencies."""
    pass

class SdistOnlyError(VendoringError):
    """Package only has source distribution, no wheel."""
    pass

class DownloadError(VendoringError):
    """Failed to download wheel."""
    pass

class PlatformMismatchError(VendoringError):
    """Downloaded wheel doesn't match target platform."""
    pass
```

### uv Installation Check

```python
# cloaca/vendoring.py

def check_uv_available() -> None:
    """Verify uv is installed and accessible."""
    try:
        result = subprocess.run(
            ["uv", "--version"],
            capture_output=True,
            text=True,
            check=True,
        )
        version = result.stdout.strip()
        print(f"Using {version}")
    except FileNotFoundError:
        raise RuntimeError(
            "uv is not installed. Install with: curl -LsSf https://astral.sh/uv/install.sh | sh"
        )
```

### Technical Dependencies

- **T-0066**: Manifest schema defines dependency list structure
- **T-0067**: Build command calls vendoring functions

### Risk Considerations

1. **Binary wheel availability**: Some packages may only have sdists. We reject these with clear errors.
2. **Platform compatibility**: manylinux tags have evolved. Support manylinux2014+ (glibc 2.17+).
3. **Cross-compilation**: Building linux packages on macos requires explicit `--platform` flags.
4. **Large vendor directories**: Some packages have many transitive deps. Consider size warnings.

## Status Updates

*To be added during implementation*
