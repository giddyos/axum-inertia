#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inertia = axum_react::build_inertia().await?;
    let app = axum_react::router(axum_react::seeded_state(), inertia);
    let address = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3003".to_owned());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("listening on http://{address}/todos");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = tokio::signal::ctrl_c();

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        result = interrupt => result.expect("install Ctrl+C handler"),
        () = terminate => {}
    }
}
