//! Axum application with Vite development and a self-contained release binary.

use axum::{Router, routing::get};
use inertia_axum::{DynamicPage, InertiaApp, RouterInertiaExt, page};

#[cfg(not(debug_assertions))]
use inertia_embed::{EmbeddedFrontend, embed_frontend};

#[cfg(not(debug_assertions))]
static FRONTEND: EmbeddedFrontend = embed_frontend! {
    root: "frontend/dist",
    entry: "src/main.ts",
    public_path: "/build",
};

async fn index() -> DynamicPage {
    page!("Home", {
        message: "Hello from one self-contained Rust binary",
    })
}

fn inertia() -> Result<InertiaApp, inertia_axum::ConfigError> {
    #[cfg(debug_assertions)]
    {
        InertiaApp::vite("frontend")
            .entry("src/main.ts")
            .dev_server("http://localhost:5173")
            .build()
    }

    #[cfg(not(debug_assertions))]
    {
        InertiaApp::embedded(&FRONTEND).build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(index))
        .with_inertia(inertia()?);
    let address = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("LISTENING {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
