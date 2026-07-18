//! Human-readable initialization output.

use crate::{
    framework::Framework,
    package_manager::PackageManager,
    server_framework::ServerFramework,
    ssr::{SsrBackend, SsrOptions},
};
use std::path::Path;

/// Formats concise post-generation next steps.
pub fn completion(
    framework: Framework,
    server: Option<ServerFramework>,
    root: &Path,
    frontend: &Path,
    manager: PackageManager,
    installed: bool,
    ssr: &SsrOptions,
) -> String {
    let name = match framework {
        Framework::React => "React",
        Framework::Svelte => "Svelte",
        Framework::Vue => "Vue",
    };
    let mut message = if let Some(server) = server {
        let frontend = frontend.strip_prefix(root).unwrap_or(frontend);
        let install = if installed {
            format!(
                "Installed frontend dependencies with {}.\n",
                manager.executable()
            )
        } else {
            format!(
                "Install frontend dependencies:\n  {} --dir {} install\n",
                manager.executable(),
                frontend.display()
            )
        };
        format!(
            "Created {name} + {} project in {}\n{install}\nDevelopment:\n  cd {}\n  cargo inertia dev\n\nSelf-contained release:\n  cd {}\n  cargo inertia build --release\n\nIf you run Cargo directly, build the frontend first:\n  {} --dir {} run build\n  cargo build --release",
            server.display_name(),
            root.display(),
            root.display(),
            root.display(),
            manager.executable(),
            frontend.display()
        )
    } else if installed {
        format!(
            "Created {name} frontend in {}\nInstalled dependencies with {}.\n\nNext:\n  {} --dir {} run check\n  cargo inertia dev",
            frontend.display(),
            manager.executable(),
            manager.executable(),
            frontend.display()
        )
    } else {
        format!(
            "Created {name} frontend in {}\n\nNext:\n  cd {}\n  {} install\n  {} run check\n  {} run build\n  cd ..\n  cargo inertia dev",
            frontend.display(),
            frontend.display(),
            manager.executable(),
            manager.executable(),
            manager.executable()
        )
    };
    if matches!(ssr.backend, SsrBackend::ManagedNode) {
        let adapter = server.map_or("inertia-axum", ServerFramework::adapter_crate);
        message.push_str(&format!(
            "\n\nEnable the {adapter} SSR feature and configure the generated server for the managed renderer at \"dist/ssr/main.js\"."
        ));
    }
    if matches!(ssr.backend, SsrBackend::ManagedNode) && matches!(manager, PackageManager::Bun) {
        message.push_str("\nBun will install dependencies and run frontend scripts. Managed SSR still requires Node 22.12 or newer at runtime.");
    }
    message
}
