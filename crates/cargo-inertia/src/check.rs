pub mod args;

use cargo_metadata::MetadataCommand;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

pub fn run(root: &Path, frontend_arg: &Path) -> Result<(), String> {
    let frontend = root.join(frontend_arg);
    if !frontend.is_dir() {
        return Err(format!(
            "frontend directory {} does not exist",
            frontend.display()
        ));
    }
    let entry = frontend.join("src/main.ts");
    if !entry.is_file() {
        return Err(format!("Vite entry {} does not exist", entry.display()));
    }
    let config = frontend.join("vite.config.ts");
    let config_text = fs::read_to_string(&config)
        .map_err(|_| format!("Vite config {} does not exist", config.display()))?;
    if !config_text.contains("src/main.ts") {
        return Err("vite.config.ts input does not match src/main.ts".into());
    }
    validate_manifest(&frontend)?;

    let sources = rust_sources(root)?;
    let mut declarations: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for source in sources {
        let text = fs::read_to_string(&source).map_err(|error| error.to_string())?;
        for component in literal_components(&text) {
            validate_component(&component)?;
            declarations
                .entry(component)
                .or_default()
                .push(source.clone());
        }
    }
    let pages = frontend.join("src/Pages");
    let frontend_components = page_components(&pages)?;
    for (component, locations) in &declarations {
        if locations.len() > 1 {
            return Err(format!(
                "duplicate component declaration `{component}` in {} files",
                locations.len()
            ));
        }
        if !frontend_components.contains(component) {
            return Err(format!(
                "component `{component}` has no matching page below {}",
                pages.display()
            ));
        }
    }
    println!(
        "cargo inertia check: {} component declarations are valid",
        declarations.len()
    );
    Ok(())
}

fn rust_sources(root: &Path) -> Result<Vec<PathBuf>, String> {
    let manifest = root.join("Cargo.toml");
    let metadata = MetadataCommand::new()
        .manifest_path(&manifest)
        .no_deps()
        .exec()
        .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let _ = metadata;
    let mut files = Vec::new();
    collect(&root, "rs", &mut files)?;
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect(directory: &Path, extension: &str, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path
            .file_name()
            .is_some_and(|name| name == "target" || name == "node_modules")
        {
            continue;
        }
        if path.is_dir() {
            collect(&path, extension, output)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            output.push(path);
        }
    }
    Ok(())
}

fn literal_components(source: &str) -> Vec<String> {
    let mut rest = source;
    let mut found = Vec::new();
    while let Some(attribute) = rest.find("#[inertia") {
        rest = &rest[attribute + 2..];
        let Some(end) = rest.find(']') else { break };
        let contents = &rest[..end];
        rest = &rest[end + 1..];
        let Some(component) = contents.find("component") else {
            continue;
        };
        let value = &contents[component + "component".len()..];
        let Some(equals) = value.find('=') else {
            continue;
        };
        let value = value[equals + 1..].trim_start();
        if let Some(value) = value.strip_prefix('"')
            && let Some(end) = value.find('"')
        {
            found.push(value[..end].to_owned());
        }
    }
    found
}

fn validate_component(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || value.split('/').any(str::is_empty)
        || value.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("invalid component path `{value}`"));
    }
    Ok(())
}

fn page_components(pages: &Path) -> Result<BTreeSet<String>, String> {
    if !pages.is_dir() {
        return Err(format!(
            "pages directory {} does not exist",
            pages.display()
        ));
    }
    let mut files = Vec::new();
    for extension in ["svelte", "tsx", "jsx", "vue"] {
        collect(pages, extension, &mut files)?;
    }
    let mut components = BTreeSet::new();
    for file in files {
        let relative = file.strip_prefix(pages).unwrap().with_extension("");
        let component = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if !components.insert(component.clone()) {
            return Err(format!("duplicate frontend page `{component}`"));
        }
    }
    Ok(components)
}

fn validate_manifest(frontend: &Path) -> Result<(), String> {
    for path in [
        frontend.join("dist/.vite/manifest.json"),
        frontend.join("dist/manifest.json"),
    ] {
        if path.is_file() {
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            let _: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|error| format!("invalid Vite manifest {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_multiline_literal_components() {
        assert_eq!(
            literal_components(
                "#[inertia(\n component = \"Todos/Index\", rename_all = \"camelCase\"\n)]"
            ),
            ["Todos/Index"]
        );
    }
    #[test]
    fn rejects_parent_paths() {
        assert!(validate_component("../Secret").is_err());
        assert!(validate_component("Todos//Index").is_err());
    }

    #[test]
    fn validates_a_complete_project_and_reports_missing_pages() {
        let root = std::env::temp_dir().join(format!("cargo-inertia-check-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("frontend/src/Pages/Todos")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='check-fixture'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "#[inertia(component = \"Todos/Index\")]\nstruct Page;\n",
        )
        .unwrap();
        fs::write(root.join("frontend/src/main.ts"), "").unwrap();
        fs::write(root.join("frontend/vite.config.ts"), "input: 'src/main.ts'").unwrap();
        fs::write(
            root.join("frontend/src/Pages/Todos/Index.svelte"),
            "<h1>Todos</h1>",
        )
        .unwrap();
        run(&root, Path::new("frontend")).unwrap();
        fs::remove_file(root.join("frontend/src/Pages/Todos/Index.svelte")).unwrap();
        assert!(
            run(&root, Path::new("frontend"))
                .unwrap_err()
                .contains("no matching page")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reports_manifest_entry_path_and_duplicate_failures() {
        let root =
            std::env::temp_dir().join(format!("cargo-inertia-check-errors-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("frontend/src/Pages/Todos")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='check-errors'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "#[inertia(component = \"Todos/Index\")]\nstruct Page;\n",
        )
        .unwrap();
        fs::write(root.join("frontend/src/main.ts"), "").unwrap();
        fs::write(root.join("frontend/vite.config.ts"), "input: 'wrong.ts'").unwrap();
        fs::write(root.join("frontend/src/Pages/Todos/Index.svelte"), "").unwrap();
        assert!(
            run(&root, Path::new("frontend"))
                .unwrap_err()
                .contains("does not match")
        );

        fs::write(root.join("frontend/vite.config.ts"), "input: 'src/main.ts'").unwrap();
        fs::create_dir_all(root.join("frontend/dist/.vite")).unwrap();
        fs::write(root.join("frontend/dist/.vite/manifest.json"), "not json").unwrap();
        assert!(
            run(&root, Path::new("frontend"))
                .unwrap_err()
                .contains("invalid Vite manifest")
        );
        fs::write(root.join("frontend/dist/.vite/manifest.json"), "{}").unwrap();

        fs::write(
            root.join("src/duplicate.rs"),
            "#[inertia(component = \"Todos/Index\")]\nstruct Duplicate;\n",
        )
        .unwrap();
        assert!(
            run(&root, Path::new("frontend"))
                .unwrap_err()
                .contains("duplicate component")
        );
        fs::remove_file(root.join("src/duplicate.rs")).unwrap();

        fs::write(
            root.join("src/invalid.rs"),
            "#[inertia(component = \"../Escape\")]\nstruct Invalid;\n",
        )
        .unwrap();
        assert!(
            run(&root, Path::new("frontend"))
                .unwrap_err()
                .contains("invalid component path")
        );
        fs::remove_file(root.join("src/invalid.rs")).unwrap();

        fs::write(root.join("frontend/src/Pages/Todos/Index.vue"), "").unwrap();
        assert!(
            run(&root, Path::new("frontend"))
                .unwrap_err()
                .contains("duplicate frontend page")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
