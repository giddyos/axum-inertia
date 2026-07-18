//! Validation of fully rendered scaffold plans.

use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

use crate::{
    error::CliError, framework::Framework, init::plan::ScaffoldPlan,
    server_framework::ServerFramework,
};

/// Validates safe paths and generated source invariants before writing anything.
pub fn validate(
    plan: &ScaffoldPlan,
    framework: Framework,
    server: Option<ServerFramework>,
) -> Result<(), CliError> {
    let mut paths = BTreeSet::new();
    for file in &plan.files {
        validate_relative_path(&file.relative_path)?;
        if !paths.insert(&file.relative_path) {
            return Err(CliError::UnsafeOutputPath(file.relative_path.clone()));
        }
        let text = std::str::from_utf8(&file.contents)
            .map_err(|_| CliError::UnsafeOutputPath(file.relative_path.clone()))?;
        if ["[[=", "[[%", "[[#"]
            .iter()
            .any(|marker| text.contains(marker))
        {
            return Err(CliError::Message(format!(
                "unrendered template marker in {}",
                file.relative_path.display()
            )));
        }
    }
    let package = plan
        .files
        .iter()
        .find(|file| file.relative_path.ends_with("package.json"))
        .ok_or_else(|| CliError::Message("missing package.json".to_owned()))?;
    let package_text = std::str::from_utf8(&package.contents)
        .map_err(|_| CliError::UnsafeOutputPath(package.relative_path.clone()))?;
    let json: serde_json::Value = serde_json::from_str(package_text)?;
    let dependencies = json.to_string();
    let selected = match framework {
        Framework::React => "@inertiajs/react",
        Framework::Svelte => "@inertiajs/svelte",
        Framework::Vue => "@inertiajs/vue3",
    };
    if !dependencies.contains(selected)
        || ["@inertiajs/react", "@inertiajs/svelte", "@inertiajs/vue3"]
            .into_iter()
            .filter(|adapter| *adapter != selected)
            .any(|adapter| dependencies.contains(adapter))
    {
        return Err(CliError::Message(
            "generated framework adapters are inconsistent".to_owned(),
        ));
    }
    let vite = plan
        .files
        .iter()
        .find(|file| file.relative_path.ends_with("vite.config.ts"))
        .ok_or_else(|| CliError::Message("missing vite.config.ts".to_owned()))?;
    let vite_text = std::str::from_utf8(&vite.contents)
        .map_err(|_| CliError::UnsafeOutputPath(vite.relative_path.clone()))?;
    if !vite_text.contains("rolldownOptions") || vite_text.contains("rollupOptions") {
        return Err(CliError::Message(
            "generated Vite configuration must use rolldownOptions".to_owned(),
        ));
    }
    if let Some(server) = server {
        validate_server_project(plan, server)?;
    }
    Ok(())
}

fn validate_server_project(plan: &ScaffoldPlan, server: ServerFramework) -> Result<(), CliError> {
    let cargo = text_file(plan, "Cargo.toml")?;
    let source = text_file(plan, "src/main.rs")?;
    let configuration = text_file(plan, "inertia.toml")?;
    let selected = server.adapter_crate();
    let adapters = ["inertia-axum", "inertia-actix", "inertia-rocket"];
    if !cargo.contains(selected)
        || adapters
            .into_iter()
            .filter(|adapter| *adapter != selected)
            .any(|adapter| cargo.contains(adapter))
        || cargo.contains("inertia-core")
    {
        return Err(CliError::Message(
            "generated Rust dependencies do not match the selected adapter".to_owned(),
        ));
    }
    for required in [
        "#[cfg(not(debug_assertions))]",
        "embed_frontend!",
        "InertiaApp::vite",
        "InertiaApp::embedded",
    ] {
        if !source.contains(required) {
            return Err(CliError::Message(format!(
                "generated server source is missing `{required}`"
            )));
        }
    }
    if matches!(server, ServerFramework::ActixWeb | ServerFramework::Rocket)
        && !source.contains(".await")
    {
        return Err(CliError::Message(
            "generated Actix Web and Rocket handlers must await rendering".to_owned(),
        ));
    }
    for required in [
        "[frontend]",
        "build-command",
        "build-dir",
        "manifest",
        "public-path",
        "[rust]",
        "package",
        "binary",
    ] {
        if !configuration.contains(required) {
            return Err(CliError::Message(format!(
                "generated inertia.toml is missing `{required}`"
            )));
        }
    }
    Ok(())
}

fn text_file<'a>(plan: &'a ScaffoldPlan, path: &str) -> Result<&'a str, CliError> {
    let file = plan
        .files
        .iter()
        .find(|file| file.relative_path == Path::new(path))
        .ok_or_else(|| CliError::Message(format!("generated project is missing {path}")))?;
    std::str::from_utf8(&file.contents)
        .map_err(|_| CliError::Message(format!("generated {path} is not UTF-8")))
}

/// Rejects output paths that could escape staging.
pub fn validate_relative_path(path: &Path) -> Result<(), CliError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        Err(CliError::UnsafeOutputPath(path.to_owned()))
    } else {
        Ok(())
    }
}
