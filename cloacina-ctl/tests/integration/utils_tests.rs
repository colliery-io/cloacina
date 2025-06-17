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

use cloacina_ctl::cli::Cli;
use cloacina_ctl::utils::*;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_cli(verbose: bool, quiet: bool) -> Cli {
    use clap::Parser;

    let mut args = vec!["cloacina-ctl", "inspect", "/test/path"];
    if verbose {
        args.insert(1, "--verbose");
    }
    if quiet {
        args.insert(1, "--quiet");
    }

    Cli::try_parse_from(args).expect("Failed to parse test CLI")
}

#[test]
fn test_should_print_error_level() {
    let cli_normal = create_test_cli(false, false);
    let cli_verbose = create_test_cli(true, false);
    let cli_quiet = create_test_cli(false, true);

    // Error level should always print
    assert!(should_print(&cli_normal, LogLevel::Error));
    assert!(should_print(&cli_verbose, LogLevel::Error));
    assert!(should_print(&cli_quiet, LogLevel::Error));
}

#[test]
fn test_should_print_info_level() {
    let cli_normal = create_test_cli(false, false);
    let cli_verbose = create_test_cli(true, false);
    let cli_quiet = create_test_cli(false, true);

    // Info level should print unless quiet
    assert!(should_print(&cli_normal, LogLevel::Info));
    assert!(should_print(&cli_verbose, LogLevel::Info));
    assert!(!should_print(&cli_quiet, LogLevel::Info));
}

#[test]
fn test_should_print_debug_level() {
    let cli_normal = create_test_cli(false, false);
    let cli_verbose = create_test_cli(true, false);
    let cli_quiet = create_test_cli(false, true);
    let cli_verbose_quiet = create_test_cli(true, true);

    // Debug level should only print when verbose and not quiet
    assert!(!should_print(&cli_normal, LogLevel::Debug));
    assert!(should_print(&cli_verbose, LogLevel::Debug));
    assert!(!should_print(&cli_quiet, LogLevel::Debug));
    assert!(!should_print(&cli_verbose_quiet, LogLevel::Debug));
}

#[test]
fn test_init_logging() {
    let cli_normal = create_test_cli(false, false);
    let cli_verbose = create_test_cli(true, false);
    let cli_quiet = create_test_cli(false, true);

    // Test that init_logging doesn't panic
    init_logging(&cli_normal);
    init_logging(&cli_verbose);
    init_logging(&cli_quiet);

    // We can't easily test the actual log level setting without
    // inspecting environment variables, but we can ensure it doesn't crash
}

#[test]
fn test_process_environment_variables_basic() {
    let mut context = json!({"existing": "value"});

    let env_vars = vec!["VAR1=value1".to_string(), "VAR2=value2".to_string()];

    let result = process_environment_variables(&mut context, &env_vars, &None, &false, &None);

    assert!(result.is_ok());
    assert_eq!(context["existing"], "value");
    assert_eq!(context["env_VAR1"], "value1");
    assert_eq!(context["env_VAR2"], "value2");
}

#[test]
fn test_process_environment_variables_with_env_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let env_file = temp_dir.path().join(".env");

    fs::write(
        &env_file,
        "FILE_VAR1=file_value1\nFILE_VAR2=file_value2\n# Comment line\n\nEMPTY_LINE_ABOVE=test",
    )
    .expect("Failed to write env file");

    let mut context = json!({});

    let result =
        process_environment_variables(&mut context, &vec![], &Some(env_file), &false, &None);

    assert!(result.is_ok());
    assert_eq!(context["env_FILE_VAR1"], "file_value1");
    assert_eq!(context["env_FILE_VAR2"], "file_value2");
    assert_eq!(context["env_EMPTY_LINE_ABOVE"], "test");
}

#[test]
fn test_process_environment_variables_include_env() {
    let mut context = json!({});

    // Set some test environment variables
    std::env::set_var("TEST_VAR1", "test_value1");
    std::env::set_var("TEST_VAR2", "test_value2");
    std::env::set_var("OTHER_VAR", "other_value");

    let result = process_environment_variables(&mut context, &vec![], &None, &true, &None);

    assert!(result.is_ok());

    // Should include all environment variables
    assert_eq!(context["env_TEST_VAR1"], "test_value1");
    assert_eq!(context["env_TEST_VAR2"], "test_value2");
    assert_eq!(context["env_OTHER_VAR"], "other_value");

    // Clean up
    std::env::remove_var("TEST_VAR1");
    std::env::remove_var("TEST_VAR2");
    std::env::remove_var("OTHER_VAR");
}

#[test]
fn test_process_environment_variables_with_prefix() {
    let mut context = json!({});

    // Set some test environment variables
    std::env::set_var("CLOACINA_VAR1", "cloacina_value1");
    std::env::set_var("CLOACINA_VAR2", "cloacina_value2");
    std::env::set_var("OTHER_VAR", "other_value");

    let result = process_environment_variables(
        &mut context,
        &vec![],
        &None,
        &true,
        &Some("CLOACINA_".to_string()),
    );

    assert!(result.is_ok());

    // Should only include variables with the prefix
    assert_eq!(context["env_CLOACINA_VAR1"], "cloacina_value1");
    assert_eq!(context["env_CLOACINA_VAR2"], "cloacina_value2");
    assert!(context.get("env_OTHER_VAR").is_none());

    // Clean up
    std::env::remove_var("CLOACINA_VAR1");
    std::env::remove_var("CLOACINA_VAR2");
    std::env::remove_var("OTHER_VAR");
}

#[test]
fn test_process_environment_variables_priority() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let env_file = temp_dir.path().join(".env");

    fs::write(&env_file, "SHARED_VAR=file_value").expect("Failed to write env file");

    std::env::set_var("SHARED_VAR", "env_value");

    let mut context = json!({});

    let env_vars = vec!["SHARED_VAR=explicit_value".to_string()];

    let result =
        process_environment_variables(&mut context, &env_vars, &Some(env_file), &true, &None);

    assert!(result.is_ok());

    // Explicit env vars should have highest priority
    assert_eq!(context["env_SHARED_VAR"], "explicit_value");

    // Clean up
    std::env::remove_var("SHARED_VAR");
}

#[test]
fn test_process_environment_variables_invalid_format() {
    let mut context = json!({});

    let env_vars = vec!["INVALID_FORMAT".to_string()]; // Missing = sign

    let result = process_environment_variables(&mut context, &env_vars, &None, &false, &None);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Invalid environment variable format"));
}

#[test]
fn test_process_environment_variables_non_object_context() {
    let mut context = json!("not an object");

    let result = process_environment_variables(&mut context, &vec![], &None, &false, &None);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Context must be a JSON object"));
}

#[test]
fn test_process_environment_variables_quoted_values() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let env_file = temp_dir.path().join(".env");

    fs::write(
        &env_file,
        r#"QUOTED_VAR="quoted value"
SINGLE_QUOTED='single quoted'
UNQUOTED=unquoted value"#,
    )
    .expect("Failed to write env file");

    let mut context = json!({});

    let result =
        process_environment_variables(&mut context, &vec![], &Some(env_file), &false, &None);

    assert!(result.is_ok());
    assert_eq!(context["env_QUOTED_VAR"], "quoted value");
    assert_eq!(context["env_SINGLE_QUOTED"], "single quoted");
    assert_eq!(context["env_UNQUOTED"], "unquoted value");
}

#[test]
fn test_process_environment_variables_nonexistent_file() {
    let mut context = json!({});
    let nonexistent_file = PathBuf::from("/nonexistent/path/.env");

    let result = process_environment_variables(
        &mut context,
        &vec![],
        &Some(nonexistent_file),
        &false,
        &None,
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to read env file"));
}
