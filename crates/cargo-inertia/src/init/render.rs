//! Strict `MiniJinja` rendering of the explicit template catalog.

use std::path::{Path, PathBuf};

use minijinja::{AutoEscape, Environment, UndefinedBehavior, syntax::SyntaxConfig};

use crate::{
    error::CliError,
    init::{
        options::InitOptions,
        plan::{RenderedFile, ScaffoldPlan},
    },
    templates::{
        catalog::{self, TEMPLATES, TemplateCondition, TemplateScope},
        context::TemplateContext,
        versions::VERSIONS,
    },
};

/// Renders every explicitly catalogued source file into memory.
pub fn render(options: &InitOptions) -> Result<ScaffoldPlan, CliError> {
    let mut environment = environment()?;
    let mut specs = catalog::for_framework(options.framework).to_vec();
    if let Some(server) = options.server_framework {
        specs.extend_from_slice(catalog::PROJECT_TEMPLATES);
        specs.extend_from_slice(catalog::for_server(server));
    }
    for spec in &specs {
        register_template(&mut environment, spec.source)?;
    }
    let project_name = cargo_package_name(
        options
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("inertia-app"),
    );
    let package_name = if options.server_framework.is_some() {
        format!("{project_name}-frontend")
    } else {
        options
            .frontend_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("frontend")
            .to_owned()
    };
    let frontend_relative = if options.server_framework.is_some() {
        options
            .frontend_dir
            .strip_prefix(&options.root)
            .ok()
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or_else(|| {
                CliError::InvalidOptions(
                    "a full-project frontend directory must be beneath the project root".to_owned(),
                )
            })?
    } else {
        Path::new("")
    };
    let frontend_dir = frontend_relative.to_str().ok_or_else(|| {
        CliError::InvalidOptions("the frontend directory must be valid UTF-8".to_owned())
    })?;
    let context = TemplateContext::new(
        options,
        &package_name,
        &project_name,
        frontend_dir,
        &VERSIONS,
    );
    let files = specs
        .iter()
        .filter(|spec| matches!(spec.condition, TemplateCondition::Always))
        .map(|spec| {
            let relative_path = match spec.scope {
                TemplateScope::Frontend if options.server_framework.is_some() => {
                    frontend_relative.join(spec.destination)
                }
                TemplateScope::Frontend | TemplateScope::Project => PathBuf::from(spec.destination),
            };
            let template =
                environment
                    .get_template(spec.source)
                    .map_err(|source| CliError::Template {
                        template: spec.source.to_owned(),
                        source,
                    })?;
            let contents = template
                .render(&context)
                .map_err(|source| CliError::Template {
                    template: spec.source.to_owned(),
                    source,
                })?;
            Ok(RenderedFile {
                relative_path,
                contents: contents.into_bytes(),
            })
        })
        .collect::<Result<Vec<_>, CliError>>()?;
    Ok(ScaffoldPlan {
        destination: if options.server_framework.is_some() {
            options.root.clone()
        } else {
            options.frontend_dir.clone()
        },
        files,
    })
}

fn cargo_package_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut separator = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            normalized.push(character.to_ascii_lowercase());
            separator = false;
        } else if !separator && !normalized.is_empty() {
            normalized.push('-');
            separator = true;
        }
    }
    while normalized.ends_with('-') {
        normalized.pop();
    }
    if normalized.is_empty() {
        "inertia-app".to_owned()
    } else {
        normalized
    }
}

fn environment() -> Result<Environment<'static>, CliError> {
    let mut environment = Environment::new();
    environment.set_syntax(
        SyntaxConfig::builder()
            .block_delimiters("[[%", "%]]")
            .variable_delimiters("[[=", "]]")
            .comment_delimiters("[[#", "#]]")
            .build()
            .map_err(CliError::TemplateSyntax)?,
    );
    environment.set_undefined_behavior(UndefinedBehavior::Strict);
    environment.set_auto_escape_callback(|_| AutoEscape::None);
    environment.set_keep_trailing_newline(true);
    Ok(environment)
}

fn register_template(
    environment: &mut Environment<'static>,
    source_name: &'static str,
) -> Result<(), CliError> {
    let file = TEMPLATES
        .get_file(source_name)
        .ok_or(CliError::MissingEmbeddedTemplate(source_name))?;
    let source = file
        .contents_utf8()
        .ok_or(CliError::TemplateIsNotUtf8(source_name))?;
    environment
        .add_template(source_name, source)
        .map_err(|source| CliError::Template {
            template: source_name.to_owned(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        framework::Framework, init::validate, package_manager::PackageManager,
        server_framework::ServerFramework, ssr::SsrOptions,
    };

    #[test]
    fn every_server_framework_renders_one_adapter_and_embedded_modes() {
        for server in [
            ServerFramework::Axum,
            ServerFramework::ActixWeb,
            ServerFramework::Rocket,
        ] {
            let root = PathBuf::from(format!("/tmp/generated-{}", server.adapter_crate()));
            let options = InitOptions {
                frontend_dir: root.join("web/client"),
                root: root.clone(),
                framework: Framework::React,
                server_framework: Some(server),
                package_manager: PackageManager::Pnpm,
                install: false,
                ssr: SsrOptions::disabled(),
            };
            let plan = render(&options).unwrap();
            validate::validate(&plan, options.framework, options.server_framework).unwrap();
            assert_eq!(plan.destination, root);
            assert!(
                plan.files.iter().any(|file| {
                    file.relative_path == Path::new("web/client/src/Pages/Home.tsx")
                })
            );
            let cargo = plan
                .files
                .iter()
                .find(|file| file.relative_path == Path::new("Cargo.toml"))
                .unwrap();
            let cargo = std::str::from_utf8(&cargo.contents).unwrap();
            assert!(cargo.contains(server.adapter_crate()));
            assert!(!cargo.contains("inertia-core"));
            assert_eq!(
                ["inertia-axum", "inertia-actix", "inertia-rocket"]
                    .into_iter()
                    .filter(|adapter| cargo.contains(adapter))
                    .count(),
                1
            );
            let source = plan
                .files
                .iter()
                .find(|file| file.relative_path == Path::new("src/main.rs"))
                .unwrap();
            let source = std::str::from_utf8(&source.contents).unwrap();
            assert!(source.contains("root: \"web/client/dist\""));
            assert!(source.contains("#[cfg(not(debug_assertions))]"));
        }
    }
}
