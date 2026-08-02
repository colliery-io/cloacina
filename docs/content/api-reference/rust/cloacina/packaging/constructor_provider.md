# cloacina::packaging::constructor_provider <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Constructor **provider package** assembly + packing (CLOACI-T-0827 / A-0011).

Turns a built `#[constructor]` **provider crate** (a *suite* of constructor
members, CLOACI-A-0011) into a distributable, signable **fidius provider
package** — the same machinery cloacina already uses to pack a workflow into a
`.cloacina` archive ([`super::package_workflow`] →
[`fidius_core::package::pack_package`]), reused for constructor providers rather
than a parallel format.

## Structs

### `cloacina::packaging::constructor_provider::ProviderPackageOptions`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Inputs to [`package_constructor_provider`].

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `crate_dir` | `PathBuf` | The `#[constructor]` provider crate directory to package. |
| `output` | `Option < PathBuf >` | Output archive path. `None` → `<name>-<version>.cloacina` in the CWD. |
| `sign_key` | `Option < PathBuf >` | Ed25519 secret-key file (32 raw bytes) to sign the package with. `None`
produces an unsigned provider. |
| `manifest_bin` | `String` | Host binary in the crate that prints the provider manifest JSON to stdout
(the `__provider_manifest()` emitter). Defaults to `emit_manifest`. |
| `release` | `bool` | Build in release profile (default `true`). |
| `runtime` | `ProviderRuntime` | Runtime to build + package for (CLOACI-T-0903). `Wasm` (default) builds a
`wasm32-wasip2` component; `Native` builds a host cdylib (loaded in-process
via the T-0902 native path). |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (crate_dir : impl Into < PathBuf >) -> Self
```

Options for `crate_dir` with the conventional defaults (`emit_manifest` bin, release build, unsigned, CWD output, WASM runtime).

<details>
<summary>Source</summary>

```rust
    pub fn new(crate_dir: impl Into<PathBuf>) -> Self {
        Self {
            crate_dir: crate_dir.into(),
            output: None,
            sign_key: None,
            manifest_bin: "emit_manifest".to_string(),
            release: true,
            runtime: ProviderRuntime::Wasm,
        }
    }
```

</details>



##### `new_native` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new_native (crate_dir : impl Into < PathBuf >) -> Self
```

As [`new`](Self::new) but building + packaging a NATIVE host cdylib (CLOACI-T-0903).

<details>
<summary>Source</summary>

```rust
    pub fn new_native(crate_dir: impl Into<PathBuf>) -> Self {
        Self {
            runtime: ProviderRuntime::Native,
            ..Self::new(crate_dir)
        }
    }
```

</details>





### `cloacina::packaging::constructor_provider::ProviderPackageResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

What [`package_constructor_provider`] produced.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `archive` | `PathBuf` | Path to the packed `.cloacina` provider archive. |
| `signed` | `bool` | Whether the archive carries a `package.sig` (was signed). |
| `provider_name` | `String` | The provider name (fidius package name). |
| `provider_version` | `String` | The provider version (from its `provider.json`). |
| `constructors` | `Vec < String >` | Names of the member constructors the provider carries. |



## Enums

### `cloacina::packaging::constructor_provider::ProviderPackageError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors assembling/packing a constructor provider package.

#### Variants

- **`Io`** - The crate directory or an expected build artifact was missing/unreadable.
- **`Build`** - `cargo build`/`cargo run` failed.
- **`Manifest`** - The emitted manifest JSON did not parse as a `ProviderManifest`, or the
provider declared no members.
- **`SigningKey`** - The Ed25519 secret key was missing or not exactly 32 bytes.
- **`Pack`** - The underlying fidius pack step failed.



## Functions

### `cloacina::packaging::constructor_provider::package_constructor_provider`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn package_constructor_provider (opts : & ProviderPackageOptions ,) -> Result < ProviderPackageResult , ProviderPackageError >
```

Build, assemble, (optionally sign,) and pack a `#[constructor]` provider crate into a distributable provider package.

Steps, mirroring `package_workflow` but for a constructor suite component:
1. Build the component per `opts.runtime`: WASM → `cargo build --lib --target
wasm32-wasip2` (a `.wasm` component); NATIVE → `cargo build --lib --features
native` (a host cdylib, CLOACI-T-0903).
2. `cargo run --bin <manifest_bin>` → the provider's manifest JSON, parsed into
a [`ProviderManifest`] (this is the packaging step writing `provider.json`,
which the macro cannot do itself).
3. Stage `package.toml`, the component, and `provider.json` (its `component`
corrected to the actual built artifact, its `runtime` stamped to
`opts.runtime`) into a temp dir. The fidius `package.toml` `runtime` is
`"wasm"` for WASM and `"rust"` for NATIVE (fidius's cdylib runtime — it has
no `"native"` value; cloacina's `native` discriminator lives in
`provider.json`).
4. If `sign_key` is set, write a `package.sig` (Ed25519 over the package
digest) reusing fidius's signing scheme.
5. [`fidius_core::package::pack_package`] → the `.cloacina` archive.

<details>
<summary>Source</summary>

```rust
pub fn package_constructor_provider(
    opts: &ProviderPackageOptions,
) -> Result<ProviderPackageResult, ProviderPackageError> {
    let crate_dir = &opts.crate_dir;
    if !crate_dir.join("Cargo.toml").exists() {
        return Err(ProviderPackageError::Io(format!(
            "no Cargo.toml in constructor provider crate dir {}",
            crate_dir.display()
        )));
    }

    // 1. Build the component: a wasm32-wasip2 component (WASM) or a host cdylib
    //    (NATIVE, CLOACI-T-0903). Both yield the file staged into the package +
    //    named in the manifest's `component`.
    let component = match opts.runtime {
        ProviderRuntime::Wasm => build_wasm_component(crate_dir, opts.release)?,
        ProviderRuntime::Native => build_native_cdylib(crate_dir, opts.release)?,
    };

    // 2. Emit + parse the provider manifest.
    let manifest_json = emit_manifest_json(crate_dir, &opts.manifest_bin, opts.runtime)?;
    let mut provider = ProviderManifest::from_json(&manifest_json)
        .map_err(|e| ProviderPackageError::Manifest(e.to_string()))?;

    let head = provider
        .constructors
        .first()
        .ok_or_else(|| {
            ProviderPackageError::Manifest(format!(
                "provider '{}' declares no constructors",
                provider.name
            ))
        })?
        .clone();

    // 3. Stage the provider package directory.
    let staging = tempfile::TempDir::new()
        .map_err(|e| ProviderPackageError::Io(format!("create staging dir: {e}")))?;
    let pkg_dir = staging.path();

    // Component filename: keep the built artifact's own name so it is stable +
    // recognizable inside the archive, and make the manifest authoritative about it.
    let component_file = component
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "provider.wasm".to_string());

    std::fs::copy(&component, pkg_dir.join(&component_file))
        .map_err(|e| ProviderPackageError::Io(format!("copy provider component: {e}")))?;

    provider.component = component_file.clone();
    // Stamp the cloacina-side runtime discriminator the loader dispatches on
    // (the macro emits Wasm by default; native packaging flips it here).
    provider.runtime = opts.runtime;

    // The provider index the loader reads (`List[Constructor]`).
    let provider_name = provider.name.clone();
    let constructor_names: Vec<String> = provider
        .constructors
        .iter()
        .map(|c| c.name.clone())
        .collect();
    let provider_json = provider
        .to_json()
        .map_err(|e| ProviderPackageError::Manifest(format!("serialize provider.json: {e}")))?;
    std::fs::write(pkg_dir.join(PROVIDER_MANIFEST_FILE), provider_json)
        .map_err(|e| ProviderPackageError::Io(format!("write provider.json: {e}")))?;

    // The fidius wasm package header. `interface` / `interface_version` come from the
    // members (a homogeneous suite shares one) so the loader's descriptor + version
    // gate line up.
    let package_toml = render_package_toml(
        &provider_name,
        &provider.version,
        &head.interface,
        head.interface_version,
        head.primitive_kind,
        &component_file,
        opts.runtime,
    );
    std::fs::write(pkg_dir.join("package.toml"), package_toml)
        .map_err(|e| ProviderPackageError::Io(format!("write package.toml: {e}")))?;

    // 4. Optional signing — must happen before packing (the .sig is archived).
    let signed = if let Some(key_path) = &opts.sign_key {
        sign_package_dir(pkg_dir, key_path)?;
        true
    } else {
        false
    };

    // 5. Pack via fidius (same path as workflow packaging).
    let result = fidius_core::package::pack_package(pkg_dir, opts.output.as_deref())
        .map_err(|e| ProviderPackageError::Pack(e.to_string()))?;

    Ok(ProviderPackageResult {
        archive: result.path,
        signed,
        provider_name,
        provider_version: provider.version.clone(),
        constructors: constructor_names,
    })
}
```

</details>



### `cloacina::packaging::constructor_provider::render_package_toml`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn render_package_toml (name : & str , version : & str , interface : & str , interface_version : u32 , primitive : PrimitiveKind , component_file : & str , runtime : ProviderRuntime ,) -> String
```

Render the fidius `package.toml` header for a constructor provider.

NOTE the TWO-manifest split (CLOACI-T-0903): the fidius `package.toml`
`runtime` vocabulary is `rust`/`python`/`wasm` — there is NO `"native"`, and
`fidius_core`'s `runtime_strict()` REJECTS unknown values. A native cdylib is
fidius's default `runtime = "rust"` (cdylib + `PluginRegistry`), which takes
no `[wasm]` section. cloacina's OWN discriminator (`wasm`/`native`) lives in
`provider.json` (`ProviderManifest.runtime`), which is what the loader
dispatches on — the loader `dlopen`s the cdylib directly and never consults
this file's runtime for native providers.

<details>
<summary>Source</summary>

```rust
fn render_package_toml(
    name: &str,
    version: &str,
    interface: &str,
    interface_version: u32,
    primitive: PrimitiveKind,
    component_file: &str,
    runtime: ProviderRuntime,
) -> String {
    let header = format!(
        "# Generated by cloacina constructor packaging (CLOACI-T-0827).\n\
         [package]\n\
         name = \"{name}\"\n\
         version = \"{version}\"\n\
         interface = \"{interface}\"\n\
         interface_version = {iface_version}\n\
         extension = \"{ext}\"\n\
         runtime = \"{fidius_runtime}\"\n\n\
         [metadata]\n\
         category = \"constructor\"\n\
         primitive_kind = \"{primitive}\"\n",
        name = name,
        version = version,
        interface = interface,
        iface_version = interface_version,
        ext = PROVIDER_EXTENSION,
        fidius_runtime = fidius_runtime_str(runtime),
        primitive = primitive_kind_str(primitive),
    );
    match runtime {
        // WASM: fidius requires the `[wasm]` section with the component filename.
        ProviderRuntime::Wasm => {
            format!("{header}\n[wasm]\ncomponent = \"{component_file}\"\n")
        }
        // NATIVE (fidius `runtime = "rust"`): no `[wasm]`/`[python]` section is
        // permitted; the cdylib filename lives in provider.json's `component`.
        ProviderRuntime::Native => header,
    }
}
```

</details>



### `cloacina::packaging::constructor_provider::fidius_runtime_str`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn fidius_runtime_str (runtime : ProviderRuntime) -> & 'static str
```

The fidius `package.toml` `runtime` string for a cloacina provider runtime. `Wasm` → `"wasm"`; `Native` → `"rust"` (fidius's cdylib runtime — it has no `"native"` value).

<details>
<summary>Source</summary>

```rust
fn fidius_runtime_str(runtime: ProviderRuntime) -> &'static str {
    match runtime {
        ProviderRuntime::Wasm => "wasm",
        ProviderRuntime::Native => "rust",
    }
}
```

</details>



### `cloacina::packaging::constructor_provider::primitive_kind_str`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn primitive_kind_str (kind : PrimitiveKind) -> & 'static str
```

<details>
<summary>Source</summary>

```rust
fn primitive_kind_str(kind: PrimitiveKind) -> &'static str {
    match kind {
        PrimitiveKind::Task => "task",
        PrimitiveKind::Trigger => "trigger",
        PrimitiveKind::Accumulator => "accumulator",
        PrimitiveKind::Reactor => "reactor",
    }
}
```

</details>



### `cloacina::packaging::constructor_provider::build_wasm_component`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn build_wasm_component (crate_dir : & Path , release : bool) -> Result < PathBuf , ProviderPackageError >
```

`cargo build --lib --target wasm32-wasip2 [--release]` in `crate_dir`, then locate the produced `.wasm` component.

Honors `CARGO_TARGET_DIR` (relative paths resolve against `crate_dir`, matching
cargo): environments like the compiler service set a SHARED target dir, so the
artifact does NOT land under `<crate>/target` there — caught live by the first
in-container provider bundle (CLOACI-T-0836).

<details>
<summary>Source</summary>

```rust
fn build_wasm_component(crate_dir: &Path, release: bool) -> Result<PathBuf, ProviderPackageError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--lib")
        .arg("--target")
        .arg("wasm32-wasip2")
        .current_dir(crate_dir);
    let profile = if release {
        cmd.arg("--release");
        "release"
    } else {
        "debug"
    };

    let status = cmd
        .status()
        .map_err(|e| ProviderPackageError::Build(format!("spawn cargo build: {e}")))?;
    if !status.success() {
        return Err(ProviderPackageError::Build(format!(
            "cargo build --target wasm32-wasip2 failed with status {status}"
        )));
    }

    // Where cargo actually wrote the artifact: CARGO_TARGET_DIR if set (relative →
    // against the build cwd, i.e. `crate_dir`), else `<crate>/target`.
    let target_root = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) if !dir.is_empty() => {
            let p = PathBuf::from(dir);
            if p.is_absolute() {
                p
            } else {
                crate_dir.join(p)
            }
        }
        _ => crate_dir.join("target"),
    };

    let out_dir = target_root.join("wasm32-wasip2").join(profile);

    // Prefer the artifact named after the crate; fall back to the sole `.wasm`.
    let preferred = crate_name(crate_dir).map(|n| out_dir.join(format!("{n}.wasm")));
    if let Some(p) = &preferred {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    let mut wasms: Vec<PathBuf> = std::fs::read_dir(&out_dir)
        .map_err(|e| ProviderPackageError::Io(format!("read {}: {e}", out_dir.display())))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("wasm"))
        .collect();
    wasms.sort();
    match wasms.len() {
        0 => Err(ProviderPackageError::Build(format!(
            "build succeeded but no .wasm component found in {}",
            out_dir.display()
        ))),
        1 => Ok(wasms.pop().unwrap()),
        _ => Err(ProviderPackageError::Build(format!(
            "multiple .wasm components in {} ({:?}); name the lib to match the crate",
            out_dir.display(),
            wasms
        ))),
    }
}
```

</details>



### `cloacina::packaging::constructor_provider::build_native_cdylib`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn build_native_cdylib (crate_dir : & Path , release : bool) -> Result < PathBuf , ProviderPackageError >
```

`cargo build --lib --features native [--release]` in `crate_dir` (CLOACI-T-0903), then locate the produced host cdylib.

The native analogue of [`build_wasm_component`]: NO `--target` (build for the
host triple), and `--features native` so the crate's native provider shell is
emitted (the `native` feature gates the `fidius_core`-referencing glue — see
the fixture Cargo.toml). Host artifacts land in `<target>/<profile>/` (no
target-triple subdir); the platform dylib is `lib<crate>.{dylib|so}` or
`<crate>.dll`. Honors `CARGO_TARGET_DIR` like the wasm path.

<details>
<summary>Source</summary>

```rust
fn build_native_cdylib(crate_dir: &Path, release: bool) -> Result<PathBuf, ProviderPackageError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--lib")
        .args(["--features", "native"])
        .current_dir(crate_dir);
    let profile = if release {
        cmd.arg("--release");
        "release"
    } else {
        "debug"
    };

    let status = cmd
        .status()
        .map_err(|e| ProviderPackageError::Build(format!("spawn cargo build (native): {e}")))?;
    if !status.success() {
        return Err(ProviderPackageError::Build(format!(
            "cargo build --lib --features native failed with status {status}"
        )));
    }

    let target_root = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) if !dir.is_empty() => {
            let p = PathBuf::from(dir);
            if p.is_absolute() {
                p
            } else {
                crate_dir.join(p)
            }
        }
        _ => crate_dir.join("target"),
    };
    let out_dir = target_root.join(profile);

    // Prefer the artifact named after the crate for each platform's convention.
    if let Some(stem) = crate_name(crate_dir) {
        for candidate in [
            format!("lib{stem}.dylib"), // macOS
            format!("lib{stem}.so"),    // Linux
            format!("{stem}.dll"),      // Windows
        ] {
            let p = out_dir.join(&candidate);
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // Fall back to the sole dynamic-library artifact in the profile dir.
    let mut libs: Vec<PathBuf> = std::fs::read_dir(&out_dir)
        .map_err(|e| ProviderPackageError::Io(format!("read {}: {e}", out_dir.display())))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|x| x.to_str()),
                Some("dylib") | Some("so") | Some("dll")
            )
        })
        .collect();
    libs.sort();
    match libs.len() {
        0 => Err(ProviderPackageError::Build(format!(
            "build succeeded but no cdylib found in {}",
            out_dir.display()
        ))),
        1 => Ok(libs.pop().unwrap()),
        _ => Err(ProviderPackageError::Build(format!(
            "multiple cdylibs in {} ({:?}); name the lib to match the crate",
            out_dir.display(),
            libs
        ))),
    }
}
```

</details>



### `cloacina::packaging::constructor_provider::crate_name`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn crate_name (crate_dir : & Path) -> Option < String >
```

Best-effort crate name (`[package].name`, `-`→`_`) for artifact matching.

<details>
<summary>Source</summary>

```rust
fn crate_name(crate_dir: &Path) -> Option<String> {
    let toml = std::fs::read_to_string(crate_dir.join("Cargo.toml")).ok()?;
    let value: toml::Value = toml.parse().ok()?;
    let name = value.get("package")?.get("name")?.as_str()?;
    Some(name.replace('-', "_"))
}
```

</details>



### `cloacina::packaging::constructor_provider::emit_manifest_json`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn emit_manifest_json (crate_dir : & Path , manifest_bin : & str , runtime : ProviderRuntime ,) -> Result < String , ProviderPackageError >
```

Run the crate's manifest-emitter host binary and capture its stdout JSON.

For a native provider the crate's native glue is behind its `native` cargo
feature, so the emitter is run with `--features native` (harmless for the
manifest itself — `__provider_manifest()` is not feature-gated — but keeps the
build unit consistent with the packaged cdylib and avoids a rebuild churn).

<details>
<summary>Source</summary>

```rust
fn emit_manifest_json(
    crate_dir: &Path,
    manifest_bin: &str,
    runtime: ProviderRuntime,
) -> Result<String, ProviderPackageError> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--quiet", "--bin", manifest_bin]);
    if runtime == ProviderRuntime::Native {
        cmd.args(["--features", "native"]);
    }
    let out = cmd.current_dir(crate_dir).output().map_err(|e| {
        ProviderPackageError::Build(format!("spawn cargo run --bin {manifest_bin}: {e}"))
    })?;
    if !out.status.success() {
        return Err(ProviderPackageError::Build(format!(
            "`cargo run --bin {manifest_bin}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    String::from_utf8(out.stdout)
        .map_err(|e| ProviderPackageError::Manifest(format!("manifest stdout not UTF-8: {e}")))
}
```

</details>



### `cloacina::packaging::constructor_provider::sign_package_dir`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn sign_package_dir (pkg_dir : & Path , key_path : & Path) -> Result < () , ProviderPackageError >
```

Sign a staged package directory in place, reusing fidius's scheme: an Ed25519 signature over [`fidius_core::package::package_digest`], written to `package.sig`. This is byte-compatible with `fidius_host::verify_package`, so we are reusing fidius's signing/verification rather than rolling our own.

<details>
<summary>Source</summary>

```rust
fn sign_package_dir(pkg_dir: &Path, key_path: &Path) -> Result<(), ProviderPackageError> {
    let key_bytes: [u8; 32] = std::fs::read(key_path)
        .map_err(|e| ProviderPackageError::SigningKey(format!("read {}: {e}", key_path.display())))?
        .try_into()
        .map_err(|_| {
            ProviderPackageError::SigningKey("secret key must be exactly 32 bytes".to_string())
        })?;
    let signing_key = SigningKey::from_bytes(&key_bytes);

    let digest = fidius_core::package::package_digest(pkg_dir)
        .map_err(|e| ProviderPackageError::Pack(format!("compute package digest: {e}")))?;
    let signature = signing_key.sign(&digest);

    std::fs::write(pkg_dir.join("package.sig"), signature.to_bytes())
        .map_err(|e| ProviderPackageError::Io(format!("write package.sig: {e}")))?;
    Ok(())
}
```

</details>
