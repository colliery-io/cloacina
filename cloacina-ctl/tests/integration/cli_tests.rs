/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use clap::Parser;
use cloacina_ctl::cli::{Cli, Commands, DebugAction, PackageCommands};
use std::path::PathBuf;

#[test]
fn test_compile_command_parsing() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "compile",
        "/path/to/project",
        "--output",
        "/path/to/output.so",
        "--",
        "--release",
        "--features=test",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse compile command");

    match cli.command {
        Commands::Package(PackageCommands::Compile {
            project_path,
            output,
            cargo_flags,
        }) => {
            assert_eq!(project_path, PathBuf::from("/path/to/project"));
            assert_eq!(output, PathBuf::from("/path/to/output.so"));
            assert_eq!(cargo_flags, vec!["--release", "--features=test"]);
        }
        _ => panic!("Expected Package Compile command"),
    }
}

#[test]
fn test_package_command_parsing() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "create",
        "/path/to/project",
        "--output",
        "/path/to/output.cloacina",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse package command");

    match cli.command {
        Commands::Package(PackageCommands::Create {
            project_path,
            output,
            cargo_flags,
        }) => {
            assert_eq!(project_path, PathBuf::from("/path/to/project"));
            assert_eq!(output, PathBuf::from("/path/to/output.cloacina"));
            assert!(cargo_flags.is_empty());
        }
        _ => panic!("Expected Package Create command"),
    }
}

#[test]
fn test_inspect_command_parsing() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "inspect",
        "/path/to/package.cloacina",
        "--format",
        "json",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse inspect command");

    match cli.command {
        Commands::Package(PackageCommands::Inspect {
            package_path,
            format,
        }) => {
            assert_eq!(package_path, PathBuf::from("/path/to/package.cloacina"));
            assert_eq!(format, "json");
        }
        _ => panic!("Expected Package Inspect command"),
    }
}

#[test]
fn test_inspect_command_default_format() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "inspect",
        "/path/to/package.cloacina",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse inspect command");

    match cli.command {
        Commands::Package(PackageCommands::Inspect {
            package_path,
            format,
        }) => {
            assert_eq!(package_path, PathBuf::from("/path/to/package.cloacina"));
            assert_eq!(format, "human"); // Default format
        }
        _ => panic!("Expected Package Inspect command"),
    }
}

#[test]
fn test_visualize_command_parsing() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "visualize",
        "/path/to/package.cloacina",
        "--details",
        "--layout",
        "compact",
        "--format",
        "dot",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse visualize command");

    match cli.command {
        Commands::Package(PackageCommands::Visualize {
            package_path,
            details,
            layout,
            format,
        }) => {
            assert_eq!(package_path, PathBuf::from("/path/to/package.cloacina"));
            assert!(details);
            assert_eq!(layout, "compact");
            assert_eq!(format, "dot");
        }
        _ => panic!("Expected Package Visualize command"),
    }
}

#[test]
fn test_visualize_command_defaults() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "visualize",
        "/path/to/package.cloacina",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse visualize command");

    match cli.command {
        Commands::Package(PackageCommands::Visualize {
            package_path,
            details,
            layout,
            format,
        }) => {
            assert_eq!(package_path, PathBuf::from("/path/to/package.cloacina"));
            assert!(!details); // Default is false
            assert_eq!(layout, "horizontal"); // Default layout
            assert_eq!(format, "ascii"); // Default format
        }
        _ => panic!("Expected Package Visualize command"),
    }
}

#[test]
fn test_debug_list_command() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "debug",
        "/path/to/package.cloacina",
        "list",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse debug list command");

    match cli.command {
        Commands::Package(PackageCommands::Debug {
            package_path,
            action,
        }) => {
            assert_eq!(package_path, PathBuf::from("/path/to/package.cloacina"));
            matches!(action, DebugAction::List);
        }
        _ => panic!("Expected Package Debug command"),
    }
}

#[test]
fn test_debug_execute_command() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "debug",
        "/path/to/package.cloacina",
        "execute",
        "task1",
        "--context",
        r#"{"key": "value"}"#,
        "--env",
        "VAR1=value1",
        "--env",
        "VAR2=value2",
        "--env-file",
        ".env",
        "--include-env",
        "--env-prefix",
        "CLOACINA_",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse debug execute command");

    match cli.command {
        Commands::Package(PackageCommands::Debug {
            package_path,
            action,
        }) => {
            assert_eq!(package_path, PathBuf::from("/path/to/package.cloacina"));
            match action {
                DebugAction::Execute {
                    task,
                    context,
                    env_vars,
                    env_file,
                    include_env,
                    env_prefix,
                } => {
                    assert_eq!(task, "task1");
                    assert_eq!(context, r#"{"key": "value"}"#);
                    assert_eq!(env_vars, vec!["VAR1=value1", "VAR2=value2"]);
                    assert_eq!(env_file, Some(PathBuf::from(".env")));
                    assert!(include_env);
                    assert_eq!(env_prefix, Some("CLOACINA_".to_string()));
                }
                _ => panic!("Expected Execute action"),
            }
        }
        _ => panic!("Expected Package Debug command"),
    }
}

#[test]
fn test_debug_execute_command_defaults() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "debug",
        "/path/to/package.cloacina",
        "execute",
        "task1",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse debug execute command");

    match cli.command {
        Commands::Package(PackageCommands::Debug {
            package_path,
            action,
        }) => {
            assert_eq!(package_path, PathBuf::from("/path/to/package.cloacina"));
            match action {
                DebugAction::Execute {
                    task,
                    context,
                    env_vars,
                    env_file,
                    include_env,
                    env_prefix,
                } => {
                    assert_eq!(task, "task1");
                    assert_eq!(context, "{}"); // Default context
                    assert!(env_vars.is_empty());
                    assert_eq!(env_file, None);
                    assert!(!include_env); // Default is false
                    assert_eq!(env_prefix, None);
                }
                _ => panic!("Expected Execute action"),
            }
        }
        _ => panic!("Expected Package Debug command"),
    }
}

#[test]
fn test_global_flags() {
    let args = vec![
        "cloacina-ctl",
        "--target",
        "x86_64-unknown-linux-gnu",
        "--profile",
        "debug",
        "--verbose",
        "--quiet",
        "--color",
        "always",
        "--jobs",
        "4",
        "package",
        "inspect",
        "/path/to/package.cloacina",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse with global flags");

    assert_eq!(cli.target, Some("x86_64-unknown-linux-gnu".to_string()));
    assert_eq!(cli.profile, "debug");
    assert!(cli.verbose);
    assert!(cli.quiet);
    assert_eq!(cli.color, "always");
    assert_eq!(cli.jobs, Some(4));
}

#[test]
fn test_global_flags_defaults() {
    let args = vec![
        "cloacina-ctl",
        "package",
        "inspect",
        "/path/to/package.cloacina",
    ];

    let cli = Cli::try_parse_from(args).expect("Should parse with default global flags");

    assert_eq!(cli.target, None);
    assert_eq!(cli.profile, "release"); // Default profile
    assert!(!cli.verbose); // Default is false
    assert!(!cli.quiet); // Default is false
    assert_eq!(cli.color, "auto"); // Default color
    assert_eq!(cli.jobs, None);
}

#[test]
fn test_invalid_command() {
    let args = vec!["cloacina-ctl", "invalid-command"];

    let result = Cli::try_parse_from(args);
    assert!(result.is_err(), "Should fail for invalid command");
}

#[test]
fn test_missing_required_args() {
    let args = vec![
        "cloacina-ctl",
        "compile", // Missing project_path and output
    ];

    let result = Cli::try_parse_from(args);
    assert!(
        result.is_err(),
        "Should fail for missing required arguments"
    );
}
