use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use graft::config::{self, expand_tilde, Install, LinkMode, PackageConfig};
use graft::error::GraftError;
use graft::link;
use graft::platform::{self, Platform};
use graft::resolve;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn verify_config(config: &config::GraftConfig) {
    // Managers
    assert_eq!(config.managers.get(&Platform::new("macos")).unwrap(), "brew install");
    assert_eq!(
        config.managers.get(&Platform::new("arch")).unwrap(),
        "pacman -S --noconfirm"
    );

    // 4 packages
    assert_eq!(config.packages.len(), 4);

    // neovim
    let nv = &config.packages["neovim"];
    assert_eq!(nv.os.as_ref().unwrap(), &[Platform::new("macos"), Platform::new("linux")]);
    assert!(matches!(&nv.install, Some(Install::Simple(s)) if s == "neovim"));
    assert_eq!(nv.tags.as_ref().unwrap(), &["editor"]);

    // ripgrep
    let rg = &config.packages["ripgrep"];
    assert_eq!(rg.link_mode, Some(LinkMode::Copy));
    assert!(matches!(&rg.install, Some(Install::PerPlatform(_))));
    assert_eq!(rg.tags.as_ref().unwrap(), &["search"]);

    // waybar
    let wb = &config.packages["waybar"];
    assert_eq!(wb.depends_on.as_ref().unwrap(), &["hyprland"]);
    assert_eq!(wb.tags.as_ref().unwrap(), &["wm"]);

    // hyprland
    let hy = &config.packages["hyprland"];
    assert_eq!(hy.os.as_ref().unwrap(), &[Platform::new("arch")]);
    assert!(matches!(&hy.install, Some(Install::Simple(s)) if s == "hyprland"));
}

#[test]
fn test_load_toml_config() {
    let (config, _) = config::load(Some(&fixture_path("graft.toml"))).unwrap();
    verify_config(&config);
}

#[test]
fn test_load_yaml_config() {
    let (config, _) = config::load(Some(&fixture_path("graft.yaml"))).unwrap();
    verify_config(&config);
}

#[test]
fn test_load_json_config() {
    let (config, _) = config::load(Some(&fixture_path("graft.json"))).unwrap();
    verify_config(&config);
}

#[test]
fn test_dependency_resolution_order() {
    let (config, _) = config::load(Some(&fixture_path("graft.toml"))).unwrap();
    let pkg_refs: HashMap<String, &PackageConfig> =
        config.packages.iter().map(|(k, v)| (k.clone(), v)).collect();

    let order =
        resolve::resolve_order(&pkg_refs, &["waybar".into()], &Platform::new("arch")).unwrap();

    let hyprland_pos = order.iter().position(|n| n == "hyprland").unwrap();
    let waybar_pos = order.iter().position(|n| n == "waybar").unwrap();
    assert!(hyprland_pos < waybar_pos);
}

#[test]
fn test_dependency_cycle_detection() {
    let a = PackageConfig {
        os: None,
        depends_on: Some(vec!["b".into()]),
        install: None,
        install_command: None,
        files: None,
        link_mode: None,
        tags: None,
    };
    let b = PackageConfig {
        os: None,
        depends_on: Some(vec!["a".into()]),
        install: None,
        install_command: None,
        files: None,
        link_mode: None,
        tags: None,
    };
    let packages: HashMap<String, &PackageConfig> =
        HashMap::from([("a".into(), &a), ("b".into(), &b)]);

    let result = resolve::resolve_order(&packages, &["a".into()], &Platform::new("macos"));
    assert!(matches!(result, Err(GraftError::CycleDetected(_))));
}

#[test]
fn test_platform_matching() {
    // Empty list matches everything
    assert!(platform::matches(&[], &Platform::new("macos")));

    // Exact match
    assert!(platform::matches(&[Platform::new("arch")], &Platform::new("arch")));
    assert!(!platform::matches(&[Platform::new("arch")], &Platform::new("macos")));

    // Linux is a catch-all for Arch and Ubuntu
    assert!(platform::matches(&[Platform::new("linux")], &Platform::new("arch")));
    assert!(platform::matches(&[Platform::new("linux")], &Platform::new("ubuntu")));
    assert!(!platform::matches(&[Platform::new("linux")], &Platform::new("macos")));

    // Multiple platforms
    assert!(platform::matches(
        &[Platform::new("macos"), Platform::new("arch")],
        &Platform::new("macos")
    ));
    assert!(!platform::matches(
        &[Platform::new("macos"), Platform::new("arch")],
        &Platform::new("ubuntu")
    ));
}

#[test]
fn test_link_and_remove() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("dotfiles");
    let dest_dir = tmp.path().join("home");

    // Create source file
    fs::create_dir_all(config_dir.join("nvim")).unwrap();
    fs::write(config_dir.join("nvim/init.lua"), "-- nvim config").unwrap();

    let dest_file = dest_dir.join(".config/nvim/init.lua");
    let dest_str = dest_file.to_str().unwrap().to_string();

    let files: HashMap<String, String> =
        HashMap::from([("nvim/init.lua".into(), dest_str.clone())]);

    // Deploy as symlink
    let errors = link::deploy_files(&files, &config_dir, LinkMode::Symlink, false, false);
    assert!(errors.is_empty(), "deploy errors: {:?}", errors);
    assert!(dest_file.exists());
    assert!(dest_file.is_symlink());

    // Verify symlink target
    let target = fs::read_link(&dest_file).unwrap();
    let expected = fs::canonicalize(config_dir.join("nvim/init.lua")).unwrap();
    assert_eq!(fs::canonicalize(target).unwrap(), expected);

    // Remove the symlink
    fs::remove_file(&dest_file).unwrap();
    assert!(!dest_file.exists());
}

#[test]
fn test_expand_tilde() {
    let home = dirs::home_dir().unwrap();

    assert_eq!(expand_tilde("~/.config/nvim"), home.join(".config/nvim"));
    assert_eq!(expand_tilde("~/"), home.join(""));
    assert_eq!(
        expand_tilde("/usr/local/bin"),
        PathBuf::from("/usr/local/bin")
    );
}
