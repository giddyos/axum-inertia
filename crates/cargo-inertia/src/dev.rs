use std::{
    path::Path,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

pub fn run(frontend: &Path, port: u16) -> Result<(), String> {
    if !frontend.join("package.json").is_file() {
        return Err(format!(
            "{} does not contain package.json",
            frontend.display()
        ));
    }
    let url = format!("http://127.0.0.1:{port}");
    let mut vite = Command::new("npm")
        .args([
            "run",
            "dev",
            "--",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .current_dir(frontend)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("could not start Vite: {error}"))?;
    let mut cargo = match Command::new("cargo")
        .arg("run")
        .env("VITE_DEV_SERVER_URL", &url)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let _ = vite.kill();
            return Err(format!("could not start cargo run: {error}"));
        }
    };
    loop {
        if let Some(status) = vite.try_wait().map_err(|error| error.to_string())? {
            let _ = cargo.kill();
            let _ = cargo.wait();
            return status
                .success()
                .then_some(())
                .ok_or_else(|| format!("Vite exited with {status}"));
        }
        if let Some(status) = cargo.try_wait().map_err(|error| error.to_string())? {
            let _ = vite.kill();
            let _ = vite.wait();
            return status
                .success()
                .then_some(())
                .ok_or_else(|| format!("cargo run exited with {status}"));
        }
        thread::sleep(Duration::from_millis(100));
    }
}
