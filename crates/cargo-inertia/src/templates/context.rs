//! Render context created exclusively from resolved initialization options.

use serde::Serialize;

use crate::{
    framework::Framework, init::options::InitOptions, server_framework::ServerFramework,
    templates::versions::TemplateVersions,
};

/// Values available to every embedded source template.
#[derive(Serialize)]
pub struct TemplateContext<'a> {
    pub package_name: &'a str,
    pub project: ProjectContext<'a>,
    pub package_manager: PackageManagerContext,
    pub server: Option<ServerContext>,
    pub framework: FrameworkContext,
    pub ssr: SsrContext<'a>,
    pub versions: &'a TemplateVersions,
}
/// Full-project path and package values.
#[derive(Serialize)]
pub struct ProjectContext<'a> {
    pub package_name: &'a str,
    pub frontend_dir: &'a str,
}
/// Selected package-manager command values.
#[derive(Serialize)]
pub struct PackageManagerContext {
    pub executable: &'static str,
}
/// Rust framework values used by project templates.
#[derive(Serialize)]
pub struct ServerContext {
    pub name: &'static str,
    pub display_name: &'static str,
    pub adapter_crate: &'static str,
}
/// Framework-specific source details.
#[derive(Serialize)]
pub struct FrameworkContext {
    pub name: &'static str,
    pub page_extension: &'static str,
    pub check_script: &'static str,
}
/// SSR values exposed to templates without any CLI parsing state.
#[derive(Serialize)]
pub struct SsrContext<'a> {
    pub enabled: bool,
    pub host: &'a str,
    pub port: u16,
    pub bundle: &'a str,
}

impl<'a> TemplateContext<'a> {
    /// Builds a render context from resolved options.
    pub fn new(
        options: &'a InitOptions,
        package_name: &'a str,
        project_name: &'a str,
        frontend_dir: &'a str,
        versions: &'a TemplateVersions,
    ) -> Self {
        let framework = match options.framework {
            Framework::React => FrameworkContext {
                name: "react",
                page_extension: "tsx",
                check_script: "tsc --noEmit",
            },
            Framework::Svelte => FrameworkContext {
                name: "svelte",
                page_extension: "svelte",
                check_script: "svelte-check",
            },
            Framework::Vue => FrameworkContext {
                name: "vue",
                page_extension: "vue",
                check_script: "vue-tsc --noEmit",
            },
        };
        let server = options.server_framework.map(|framework| {
            let name = match framework {
                ServerFramework::Axum => "axum",
                ServerFramework::ActixWeb => "actix-web",
                ServerFramework::Rocket => "rocket",
            };
            ServerContext {
                name,
                display_name: framework.display_name(),
                adapter_crate: framework.adapter_crate(),
            }
        });
        Self {
            package_name,
            project: ProjectContext {
                package_name: project_name,
                frontend_dir,
            },
            package_manager: PackageManagerContext {
                executable: options.package_manager.executable(),
            },
            server,
            framework,
            ssr: SsrContext {
                enabled: options.ssr.is_enabled(),
                host: &options.ssr.host,
                port: options.ssr.port,
                bundle: options.ssr.bundle.to_str().unwrap_or("dist/ssr/main.js"),
            },
            versions,
        }
    }
}
