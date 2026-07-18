#![allow(missing_docs)]
#![cfg(feature = "init")]

use std::{fs, path::Path, process::Command};

use cargo_inertia::{
    framework::Framework,
    init::{options::InitOptions, render, validate, write},
    package_manager::PackageManager,
    server_framework::ServerFramework,
    ssr::SsrOptions,
};

#[test]
fn generated_debug_and_embedded_release_projects_compile_for_every_adapter() {
    let fixture = assert_fs::TempDir::new().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_owned();
    let target = repository.join("target/generated-projects");

    for server in [
        ServerFramework::Axum,
        ServerFramework::ActixWeb,
        ServerFramework::Rocket,
    ] {
        let name = server.adapter_crate().replace("inertia-", "generated-");
        let root = fixture.path().join(name);
        let options = InitOptions {
            root: root.clone(),
            frontend_dir: root.join("frontend"),
            framework: Framework::React,
            server_framework: Some(server),
            package_manager: PackageManager::Pnpm,
            install: false,
            ssr: SsrOptions::disabled(),
        };
        let plan = render::render(&options).unwrap();
        validate::validate(&plan, options.framework, options.server_framework).unwrap();
        write::commit(&plan).unwrap();
        write_embedded_fixture(&root);
        write_patch_configuration(&root, &repository);

        check_generated(&root, &target, false);
        check_generated(&root, &target, true);
    }
}

fn write_embedded_fixture(root: &Path) {
    let dist = root.join("frontend/dist");
    fs::create_dir_all(dist.join(".vite")).unwrap();
    fs::create_dir_all(dist.join("assets")).unwrap();
    fs::write(dist.join("assets/app.js"), "console.log('generated')").unwrap();
    fs::write(dist.join("assets/app.css"), "body{color:#123}").unwrap();
    fs::write(
        dist.join(".vite/manifest.json"),
        r#"{
  "src/main.ts": {
    "file": "assets/app.js",
    "css": ["assets/app.css"],
    "isEntry": true
  }
}"#,
    )
    .unwrap();
}

fn write_patch_configuration(root: &Path, repository: &Path) {
    let cargo = root.join(".cargo");
    fs::create_dir_all(&cargo).unwrap();
    let path = |relative: &str| toml_path(&repository.join(relative));
    fs::write(
        cargo.join("config.toml"),
        format!(
            r#"[patch.crates-io]
inertia-core = {{ path = "{}" }}
inertia-axum = {{ path = "{}" }}
inertia-actix = {{ path = "{}" }}
inertia-rocket = {{ path = "{}" }}
inertia-embed = {{ path = "{}" }}
inertia-embed-macros = {{ path = "{}" }}
inertia-macros = {{ path = "{}" }}
inertia-typegen = {{ path = "{}" }}
"#,
            path("crates/inertia-core"),
            path("crates/inertia-axum"),
            path("crates/inertia-actix"),
            path("crates/inertia-rocket"),
            path("crates/inertia-embed"),
            path("crates/inertia-embed/macros"),
            path("crates/inertia-macros"),
            path("crates/inertia-typegen"),
        ),
    )
    .unwrap();
}

fn check_generated(root: &Path, target: &Path, release: bool) {
    let mut command = Command::new(env!("CARGO"));
    command
        .arg("check")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .current_dir(root)
        .env("CARGO_TARGET_DIR", target);
    if release {
        command.arg("--release");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{} generated project failed to compile in {} mode\nstdout:\n{}\nstderr:\n{}",
        root.display(),
        if release { "release" } else { "debug" },
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
