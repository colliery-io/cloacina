#!/usr/bin/env python3
"""
Cloacina Desktop App Development Commands
"""
import subprocess
import sys
from pathlib import Path

import angreal # type: ignore


# Define command group
app = angreal.command_group(name="app", about="commands for Cloacina Desktop App development")


@app()
@angreal.command(name="dev")
def dev():
    """
    Start Tauri development server with hot reload

    When to use:
    - Active frontend/backend development
    - Testing UI changes with live reload
    - Debugging Tauri commands

    When not to use:
    - Production builds
    - Mobile development (use app android/ios instead)
    """
    print("Starting Cloacina Desktop App development server...")

    # Change to cloacina-app directory
    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)


    try:
        # Start Tauri development server with debug logging
        # Don't capture stdout/stderr - let them pass through to terminal
        result = subprocess.run(
            ["cargo", "tauri", "dev"],
            check=True,
            cwd=app_dir,
            stdout=None,  # Pass through to terminal
            stderr=None   # Pass through to terminal
        )
    except subprocess.CalledProcessError as e:
        print(f"Development server failed: {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\nDevelopment server stopped")


@app()
@angreal.command(name="build")
def build():
    """
    Build Tauri application for production

    When to use:
    - Creating production releases
    - Testing performance optimizations
    - Generating distributable packages

    When not to use:
    - Development testing (use app dev)
    - Quick iterations
    """
    print("Building Cloacina Desktop App for production...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    try:
        result = subprocess.run(
            ["cargo", "tauri", "build"],
            check=True,
            cwd=app_dir
        )
        print("Production build completed!")
    except subprocess.CalledProcessError as e:
        print(f"Build failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="test")
def test():
    """
    Run all tests for the Tauri application

    When to use:
    - Before committing changes
    - CI/CD validation
    - Testing Tauri command functionality

    When not to use:
    - Core cloacina library testing (use cloacina unit/integration)
    """
    print("Running Cloacina Desktop App tests...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    try:
        # Run Rust tests with verbose output
        print("🧪 Running Rust unit tests...")
        result = subprocess.run(
            ["cargo", "test", "--verbose"],
            check=True,
            cwd=app_dir / "src-tauri"
        )
        print("✅ All tests passed!")

    except subprocess.CalledProcessError as e:
        print(f"❌ Tests failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="test-unit")
def test_unit():
    """
    Run only unit tests (fast)

    When to use:
    - Quick validation during development
    - Testing specific modules
    - TDD workflow

    When not to use:
    - Full validation before commit (use app test)
    """
    print("Running unit tests only...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    try:
        # Run only library tests (no integration tests)
        print("🧪 Running unit tests...")
        result = subprocess.run(
            ["cargo", "test", "--lib"],
            check=True,
            cwd=app_dir / "src-tauri"
        )
        print("✅ Unit tests passed!")

    except subprocess.CalledProcessError as e:
        print(f"❌ Unit tests failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="test-watch")
def test_watch():
    """
    Run tests in watch mode (re-run on file changes)

    When to use:
    - Active TDD development
    - Continuous testing during coding
    - Debugging failing tests

    When not to use:
    - CI/CD environments
    - Final validation
    """
    print("Starting test watch mode...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    # Check if cargo-watch is installed
    try:
        subprocess.run(["cargo", "watch", "--version"],
                      check=True, capture_output=True)
    except subprocess.CalledProcessError:
        print("Installing cargo-watch...")
        try:
            subprocess.run(["cargo", "install", "cargo-watch"], check=True)
        except subprocess.CalledProcessError as e:
            print(f"Failed to install cargo-watch: {e}")
            sys.exit(1)

    try:
        print("🧪 Starting test watch mode (Ctrl+C to stop)...")
        result = subprocess.run(
            ["cargo", "watch", "-x", "test"],
            check=True,
            cwd=app_dir / "src-tauri"
        )

    except KeyboardInterrupt:
        print("\n🛑 Test watch stopped")
    except subprocess.CalledProcessError as e:
        print(f"❌ Test watch failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="lint")
def lint():
    """
    Run linting and formatting checks

    When to use:
    - Before committing code
    - Code quality validation
    - CI/CD checks

    When not to use:
    - Quick development iteration
    """
    print("Running linting and formatting checks...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    tauri_dir = app_dir / "src-tauri"

    try:
        # Check Rust formatting
        print("🔍 Checking Rust formatting...")
        result = subprocess.run(
            ["cargo", "fmt", "--check"],
            check=True,
            cwd=tauri_dir
        )
        print("✅ Rust formatting OK")

        # Run Clippy
        print("🔍 Running Clippy...")
        result = subprocess.run(
            ["cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"],
            check=True,
            cwd=tauri_dir
        )
        print("✅ Clippy checks passed")

        print("✅ All linting checks passed!")

    except subprocess.CalledProcessError as e:
        print(f"❌ Linting failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="fix")
def fix():
    """
    Automatically fix formatting and some linting issues

    When to use:
    - Before committing
    - Cleaning up code style
    - Fixing automatic linting issues

    When not to use:
    - When you want to manually review changes
    """
    print("Fixing formatting and linting issues...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    tauri_dir = app_dir / "src-tauri"

    try:
        # Format Rust code
        print("🔧 Formatting Rust code...")
        subprocess.run(["cargo", "fmt"], check=True, cwd=tauri_dir)
        print("✅ Rust code formatted")

        # Fix Clippy issues (where possible)
        print("🔧 Fixing Clippy issues...")
        subprocess.run(["cargo", "clippy", "--fix", "--allow-dirty"],
                      check=True, cwd=tauri_dir)
        print("✅ Clippy fixes applied")

        print("✅ Code fixes completed!")

    except subprocess.CalledProcessError as e:
        print(f"❌ Fix failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="android")
def android():
    """
    Start Android development server

    When to use:
    - Mobile development and testing
    - Android-specific UI testing
    - Touch interface validation

    When not to use:
    - Desktop development
    - Initial prototyping (use app dev)
    """
    print("Starting Android development server...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    try:
        # Check if Android is initialized
        if not (app_dir / "src-tauri" / "gen" / "android").exists():
            print("Initializing Android development...")
            subprocess.run(["cargo", "tauri", "android", "init"], check=True, cwd=app_dir)

        # Start Android dev server
        result = subprocess.run(
            ["cargo", "tauri", "android", "dev"],
            check=True,
            cwd=app_dir
        )
    except subprocess.CalledProcessError as e:
        print(f"Android development failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="ios")
def ios():
    """
    Start iOS development server

    When to use:
    - iOS development and testing
    - iPhone/iPad interface validation
    - Touch interface optimization

    When not to use:
    - Non-macOS development machines
    - Desktop-focused development
    """
    print("Starting iOS development server...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    try:
        # Check if iOS is initialized
        if not (app_dir / "src-tauri" / "gen" / "apple").exists():
            print("Initializing iOS development...")
            subprocess.run(["cargo", "tauri", "ios", "init"], check=True, cwd=app_dir)

        # Start iOS dev server
        result = subprocess.run(
            ["cargo", "tauri", "ios", "dev"],
            check=True,
            cwd=app_dir
        )
    except subprocess.CalledProcessError as e:
        print(f"iOS development failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="setup")
def setup():
    """
    Initialize mobile development platforms

    When to use:
    - First time mobile development setup
    - Adding mobile support to existing app
    - Resetting mobile configurations

    When not to use:
    - After mobile platforms are already configured
    - Desktop-only development
    """
    print("Setting up mobile development platforms...")

    app_dir = Path.cwd() / "cloacina-app"
    if not app_dir.exists():
        print("cloacina-app directory not found!")
        sys.exit(1)

    try:
        # Initialize Android
        print("Initializing Android...")
        subprocess.run(["cargo", "tauri", "android", "init"], check=True, cwd=app_dir)

        # Initialize iOS (macOS only)
        import platform
        if platform.system() == "Darwin":
            print("Initializing iOS...")
            subprocess.run(["cargo", "tauri", "ios", "init"], check=True, cwd=app_dir)
        else:
            print("iOS development only available on macOS")

        print("Mobile platforms initialized!")

    except subprocess.CalledProcessError as e:
        print(f"Setup failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="deps")
def deps():
    """
    Install and update Tauri CLI dependencies

    When to use:
    - First time setup
    - Updating to newer Tauri versions
    - Fixing CLI installation issues

    When not to use:
    - Regular development (dependencies should be stable)
    """
    print("Installing/updating Tauri CLI dependencies...")

    try:
        # Install Tauri CLI
        print("Installing Tauri CLI...")
        subprocess.run([
            "cargo", "install", "tauri-cli",
            "--version", "^2.0.0", "--locked"
        ], check=True)

        print("Tauri CLI installed successfully!")

    except subprocess.CalledProcessError as e:
        print(f"Dependency installation failed: {e}")
        sys.exit(1)


@app()
@angreal.command(name="clean")
def clean():
    """
    Clean application data for fresh development restart

    When to use:
    - Development testing with clean slate
    - After schema/settings changes
    - Troubleshooting data corruption issues
    - Testing application initialization

    When not to use:
    - Production environments
    - When you want to preserve development data

    WARNING: This will delete ALL application data including:
    - Settings configuration
    - Application database
    - Log files
    - Runner configurations
    """
    import platform
    from pathlib import Path

    print("🚨 WARNING: This will delete ALL application data!")
    print("This includes:")
    print("  - Settings configuration")
    print("  - Application database")
    print("  - Log files")
    print("  - Runner configurations")
    print()

    # Get confirmation
    response = input("Are you sure you want to continue? (type 'yes' to confirm): ")
    if response.lower() != 'yes':
        print("Clean operation cancelled.")
        return

    # Determine data directories based on OS
    system = platform.system()

    if system == "Darwin":  # macOS
        data_dir = Path.home() / "Library" / "Application Support" / "Cloacina"
        config_dir = Path.home() / "Library" / "Application Support" / "Cloacina"
    elif system == "Windows":
        data_dir = Path.home() / "AppData" / "Roaming" / "Cloacina"
        config_dir = Path.home() / "AppData" / "Roaming" / "Cloacina"
    else:  # Linux and others
        data_dir = Path.home() / ".local" / "share" / "cloacina"
        config_dir = Path.home() / ".config" / "Cloacina"

    cleaned_items = []

    try:
        # Clean data directory (database, logs)
        if data_dir.exists():
            import shutil
            shutil.rmtree(data_dir)
            cleaned_items.append(f"Data directory: {data_dir}")

        # Clean config directory (settings) if different from data dir
        if config_dir != data_dir and config_dir.exists():
            import shutil
            shutil.rmtree(config_dir)
            cleaned_items.append(f"Config directory: {config_dir}")

        if cleaned_items:
            print("✅ Successfully cleaned:")
            for item in cleaned_items:
                print(f"   {item}")
        else:
            print("ℹ️  No application data found to clean.")

        print()
        print("Application will start fresh on next launch.")

    except Exception as e:
        print(f"❌ Error during cleanup: {e}")
        sys.exit(1)
