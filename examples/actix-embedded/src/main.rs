//! Actix Web application with Vite development and embedded release assets.

use actix_web::{App, HttpServer, web};
use inertia_actix::{Inertia, InertiaApp, InertiaMiddleware, Result as InertiaResult, assets};
use serde::Serialize;
use std::io::Write as _;

#[cfg(not(debug_assertions))]
use inertia_embed::{EmbeddedFrontend, embed_frontend};

#[cfg(not(debug_assertions))]
static FRONTEND: EmbeddedFrontend = embed_frontend! {
    root: "frontend/dist",
    entry: "src/main.ts",
    public_path: "/build",
};

#[derive(Serialize)]
struct HomeProps {
    message: &'static str,
}

async fn index(inertia: Inertia) -> InertiaResult {
    inertia
        .render(
            "Home",
            HomeProps {
                message: "Hello from one self-contained Actix Web binary",
            },
        )
        .await
}

fn inertia() -> Result<InertiaApp, inertia_actix::ConfigError> {
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let inertia = inertia().map_err(std::io::Error::other)?;
    let address = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());

    let server = HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(index))
            .app_data(web::Data::new(inertia.clone()))
            .wrap(InertiaMiddleware::new(inertia.clone()))
            .configure(assets(inertia.clone()))
    })
    .bind(address)?;
    let bound = server
        .addrs()
        .first()
        .copied()
        .ok_or_else(|| std::io::Error::other("Actix Web did not bind a listener"))?;
    println!("LISTENING {bound}");
    std::io::stdout().flush()?;
    server.run().await
}
