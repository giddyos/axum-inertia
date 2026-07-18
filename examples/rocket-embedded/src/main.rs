//! Rocket application with Vite development and embedded release assets.

use inertia_rocket::{Inertia, InertiaApp, InertiaFairing, Result as InertiaResult};
use rocket::fairing::AdHoc;
use serde::Serialize;
use std::{io::Write as _, net::SocketAddr};

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

#[rocket::get("/")]
async fn index(inertia: Inertia<'_>) -> InertiaResult {
    inertia
        .render(
            "Home",
            HomeProps {
                message: "Hello from one self-contained Rocket binary",
            },
        )
        .await
}

fn inertia() -> Result<InertiaApp, inertia_rocket::ConfigError> {
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

#[rocket::launch]
fn rocket() -> _ {
    let inertia = inertia().expect("valid Inertia configuration");
    let address = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let address = address
        .parse::<SocketAddr>()
        .expect("ADDR must contain an IP socket address");
    let figment = rocket::Config::figment()
        .merge(("address", address.ip()))
        .merge(("port", address.port()));

    rocket::custom(figment)
        .attach(InertiaFairing::new(inertia))
        .attach(AdHoc::on_liftoff("Report listening address", |rocket| {
            Box::pin(async move {
                println!(
                    "LISTENING {}:{}",
                    rocket.config().address,
                    rocket.config().port
                );
                std::io::stdout()
                    .flush()
                    .expect("listening address must flush");
            })
        }))
        .mount("/", rocket::routes![index])
}
