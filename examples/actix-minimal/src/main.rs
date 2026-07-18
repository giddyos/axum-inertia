//! Minimal Actix Web application using the framework-neutral Inertia runtime.

use actix_web::{App, HttpServer, web};
use inertia_actix::{DynamicPage, InertiaApp, InertiaMiddleware, assets, page};

async fn index() -> DynamicPage {
    page!("Home", {
        message: "Rendered by Actix Web through Inertia.",
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let inertia = InertiaApp::vite("frontend")
        .dev_server("http://localhost:5173")
        .build()
        .map_err(std::io::Error::other)?;
    let address = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3001".to_owned());

    HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(index))
            .app_data(web::Data::new(inertia.clone()))
            .wrap(InertiaMiddleware::new(inertia.clone()))
            .configure(assets(inertia.clone()))
    })
    .bind(address)?
    .run()
    .await
}
