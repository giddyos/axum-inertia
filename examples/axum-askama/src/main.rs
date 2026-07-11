//! Minimal Axum application using an Askama-compiled Inertia root document.

mod root;

use axum::{Router, routing::get};
use inertia_axum::prelude::*;
use root::AppRoot;
use std::path::PathBuf;

async fn home() -> DynamicPage {
    page!("Home", {
        message: "Rendered by Axum through Inertia.",
    })
}

fn router(inertia: InertiaApp) -> Router {
    Router::new().route("/", get(home)).inertia(inertia)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let inertia = InertiaApp::vite(project_root.join("frontend"))
        .askama_root(AppRoot::new(
            "Askama + Inertia",
            "An Askama root document with Inertia and Axum.",
        ))
        .build()?;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    axum::serve(listener, router(inertia)).await?;

    Ok(())
}
