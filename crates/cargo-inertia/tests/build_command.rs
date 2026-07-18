#![allow(missing_docs)]
#![cfg(all(unix, feature = "build"))]

use std::{fs, os::unix::fs::PermissionsExt as _, path::Path};

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn frontend_build_precedes_cargo_and_the_reported_executable_is_real() {
    let project = assert_fs::TempDir::new().unwrap();
    write_project(project.path());
    let target = project.path().join("target");
    let executable = target.join("release/server");

    Command::cargo_bin("cargo-inertia")
        .unwrap()
        .current_dir(project.path())
        .env("CARGO_TARGET_DIR", &target)
        .args([
            "build",
            "--release",
            "--package",
            "server",
            "--",
            "--features",
            "forwarded",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Embedded frontend: 2 assets, "))
        .stdout(predicate::str::contains(format!(
            "Executable: {}",
            executable.display()
        )));

    assert!(executable.is_file());
    assert_eq!(
        fs::read_to_string(project.path().join("frontend/build-order.log")).unwrap(),
        "frontend\ncargo\n"
    );
}

fn write_project(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("frontend")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "server"
version = "0.1.0"
edition = "2024"
build = "build.rs"

[features]
forwarded = []
"#,
    )
    .unwrap();
    fs::write(
        root.join("build.rs"),
        r#"fn main() {
    let log = std::path::Path::new("frontend/build-order.log");
    if !std::path::Path::new("frontend/dist/.vite/manifest.json").is_file() {
        panic!("frontend manifest must exist before Cargo starts");
    }
    let mut options = std::fs::OpenOptions::new();
    options.append(true);
    use std::io::Write as _;
    writeln!(options.open(log).unwrap(), "cargo").unwrap();
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.rs"),
        r#"#[cfg(not(feature = "forwarded"))]
compile_error!("arguments after -- were not forwarded to Cargo");

fn main() {}
"#,
    )
    .unwrap();
    fs::write(
        root.join("inertia.toml"),
        r#"[frontend]
root = "frontend"
build-command = ["./build-frontend", "--production"]
build-dir = "frontend/dist"
manifest = "frontend/dist/.vite/manifest.json"
entry = "src/main.ts"
public-path = "/build"

[rust]
package = "server"
binary = "server"
"#,
    )
    .unwrap();
    let script = root.join("frontend/build-frontend");
    fs::write(
        &script,
        r#"#!/usr/bin/env sh
set -eu
test "${1:-}" = "--production"
mkdir -p dist/.vite dist/assets
printf '%s\n' 'export default 1' >dist/assets/main.js
printf '%s\n' '{"src/main.ts":{"file":"assets/main.js","isEntry":true}}' >dist/.vite/manifest.json
printf '%s\n' frontend >build-order.log
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(script, permissions).unwrap();
}
