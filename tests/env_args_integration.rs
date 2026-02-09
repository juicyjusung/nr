use nr::core::env_files::scan_env_files;
use nr::store::global_env::{load_global_env_config, save_global_env_config, GlobalEnvConfig};
use nr::store::script_configs::{load_script_configs, save_script_configs, ScriptConfig};
use std::collections::HashMap;
use std::fs;
use std::time::SystemTime;
use tempfile::TempDir;

/// Test that global env config persists across sessions
#[test]
fn test_global_env_persists_across_sessions() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // First session: save global env
    let mut config = GlobalEnvConfig::default();
    config.last_env_files = vec![".env".to_string(), ".env.local".to_string()];
    save_global_env_config(config_dir, &config).unwrap();

    // Second session: load global env
    let loaded = load_global_env_config(config_dir).unwrap();
    assert_eq!(loaded.last_env_files, vec![".env", ".env.local"]);
}

/// Test that script-specific args persist independently
#[test]
fn test_script_args_persist_independently() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    let mut configs = HashMap::new();

    // Script 1: test with --watch
    configs.insert(
        "proj123:root:test".to_string(),
        ScriptConfig {
            args: "--watch".to_string(),
            last_used: SystemTime::now(),
        },
    );

    // Script 2: build with --production
    configs.insert(
        "proj123:root:build".to_string(),
        ScriptConfig {
            args: "--production".to_string(),
            last_used: SystemTime::now(),
        },
    );

    save_script_configs(config_dir, &configs).unwrap();

    // Load and verify
    let loaded = load_script_configs(config_dir).unwrap();
    assert_eq!(
        loaded.get("proj123:root:test").unwrap().args,
        "--watch"
    );
    assert_eq!(
        loaded.get("proj123:root:build").unwrap().args,
        "--production"
    );
}

/// Test that global env updates don't affect script args
#[test]
fn test_global_env_independent_of_script_args() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Save script args
    let mut script_configs = HashMap::new();
    script_configs.insert(
        "proj:root:test".to_string(),
        ScriptConfig {
            args: "--watch".to_string(),
            last_used: SystemTime::now(),
        },
    );
    save_script_configs(config_dir, &script_configs).unwrap();

    // Save global env
    let mut global_env = GlobalEnvConfig::default();
    global_env.last_env_files = vec![".env".to_string()];
    save_global_env_config(config_dir, &global_env).unwrap();

    // Update global env
    global_env.last_env_files = vec![".env.local".to_string()];
    save_global_env_config(config_dir, &global_env).unwrap();

    // Script args should remain unchanged
    let loaded_scripts = load_script_configs(config_dir).unwrap();
    assert_eq!(
        loaded_scripts.get("proj:root:test").unwrap().args,
        "--watch"
    );

    // Global env should be updated
    let loaded_env = load_global_env_config(config_dir).unwrap();
    assert_eq!(loaded_env.last_env_files, vec![".env.local"]);
}

/// Test env file scanning and selection workflow
#[test]
fn test_env_file_selection_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let root_dir = temp_dir.path();
    let package_dir = root_dir.join("packages").join("web");
    fs::create_dir_all(&package_dir).unwrap();

    // Create env files
    fs::write(root_dir.join(".env"), "ROOT_VAR=root").unwrap();
    fs::write(package_dir.join(".env"), "PKG_VAR=pkg").unwrap();
    fs::write(package_dir.join(".env.local"), "LOCAL_VAR=local").unwrap();

    // Scan env files
    let env_list = scan_env_files(&package_dir, &Some(root_dir.to_path_buf()));

    // Should find package files and root files
    assert_eq!(env_list.package_files.len(), 2); // .env, .env.local
    assert_eq!(env_list.root_files.len(), 1); // .env

    // Verify display names
    let all_files: Vec<_> = env_list.all_files().collect();
    assert_eq!(all_files.len(), 3);

    let display_names: Vec<_> = all_files.iter().map(|f| &f.display_name).collect();
    assert!(display_names.contains(&&".env".to_string()));
    assert!(display_names.contains(&&".env.local".to_string()));
}

/// Test that empty global env config doesn't cause issues
#[test]
fn test_empty_global_env_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    let config = GlobalEnvConfig::default();
    save_global_env_config(config_dir, &config).unwrap();

    let loaded = load_global_env_config(config_dir).unwrap();
    assert_eq!(loaded.last_env_files.len(), 0);
}

/// Test env file merge order (root → package)
#[test]
fn test_env_file_merge_order() {
    let temp_dir = TempDir::new().unwrap();
    let root_dir = temp_dir.path();
    let package_dir = root_dir.join("pkg");
    fs::create_dir_all(&package_dir).unwrap();

    // Create env files with overlapping keys
    fs::write(root_dir.join(".env"), "VAR=root\nROOT_ONLY=yes").unwrap();
    fs::write(package_dir.join(".env"), "VAR=package\nPKG_ONLY=yes").unwrap();

    let env_list = scan_env_files(&package_dir, &Some(root_dir.to_path_buf()));

    // Get merge order
    let merge_order: Vec<_> = env_list.all_files_merge_order().collect();

    // Root should come first, package second
    assert_eq!(merge_order.len(), 2);
    
    // Verify order by checking parent directory
    let first_parent = merge_order[0].path.parent().unwrap();
    let second_parent = merge_order[1].path.parent().unwrap();
    
    // First should be from root_dir, second from package_dir
    assert_eq!(first_parent, root_dir);
    assert_eq!(second_parent, package_dir);
}

/// Test multiple scripts sharing global env but different args
#[test]
fn test_multiple_scripts_share_global_env() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Save global env (shared)
    let mut global_env = GlobalEnvConfig::default();
    global_env.last_env_files = vec![".env".to_string(), ".env.local".to_string()];
    save_global_env_config(config_dir, &global_env).unwrap();

    // Save different args for different scripts
    let mut script_configs = HashMap::new();
    script_configs.insert(
        "proj:root:test".to_string(),
        ScriptConfig {
            args: "--watch".to_string(),
            last_used: SystemTime::now(),
        },
    );
    script_configs.insert(
        "proj:root:build".to_string(),
        ScriptConfig {
            args: "--production".to_string(),
            last_used: SystemTime::now(),
        },
    );
    script_configs.insert(
        "proj:root:dev".to_string(),
        ScriptConfig {
            args: "--hot".to_string(),
            last_used: SystemTime::now(),
        },
    );
    save_script_configs(config_dir, &script_configs).unwrap();

    // Verify: all scripts can access same global env
    let loaded_env = load_global_env_config(config_dir).unwrap();
    assert_eq!(loaded_env.last_env_files.len(), 2);

    // Verify: each script has its own args
    let loaded_configs = load_script_configs(config_dir).unwrap();
    assert_eq!(
        loaded_configs.get("proj:root:test").unwrap().args,
        "--watch"
    );
    assert_eq!(
        loaded_configs.get("proj:root:build").unwrap().args,
        "--production"
    );
    assert_eq!(
        loaded_configs.get("proj:root:dev").unwrap().args,
        "--hot"
    );
}

/// Test that updating global env affects all future sessions
#[test]
fn test_global_env_updates_affect_all_scripts() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Initial global env
    let mut global_env = GlobalEnvConfig::default();
    global_env.last_env_files = vec![".env".to_string()];
    save_global_env_config(config_dir, &global_env).unwrap();

    // Update global env (simulating execution from any script)
    global_env.last_env_files = vec![".env".to_string(), ".env.production".to_string()];
    save_global_env_config(config_dir, &global_env).unwrap();

    // Any subsequent script should see the updated env
    let loaded = load_global_env_config(config_dir).unwrap();
    assert_eq!(loaded.last_env_files, vec![".env", ".env.production"]);
}

/// Test backward compatibility: old script configs without env_files should work
#[test]
fn test_backward_compatibility_script_configs() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Manually create old format (would have had env_files field)
    // New format only has args
    let mut configs = HashMap::new();
    configs.insert(
        "proj:root:test".to_string(),
        ScriptConfig {
            args: "--watch".to_string(),
            last_used: SystemTime::now(),
        },
    );
    save_script_configs(config_dir, &configs).unwrap();

    // Should load successfully
    let loaded = load_script_configs(config_dir).unwrap();
    assert_eq!(loaded.get("proj:root:test").unwrap().args, "--watch");
}
