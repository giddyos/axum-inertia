use crate::{
    error::CliError,
    package_manager::{CommandSpec, PackageManager},
};
use std::{
    ffi::OsString,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

pub struct DevPlan {
    pub frontend: CommandSpec,
    pub cargo: CommandSpec,
    pub vite_url: String,
    pub readiness_address: SocketAddr,
    pub readiness_timeout: Duration,
}

pub fn build_dev_plan(
    frontend: PathBuf,
    manager: PackageManager,
    host: &str,
    port: u16,
    cargo_args: Vec<OsString>,
) -> Result<DevPlan, CliError> {
    let vite_url = format!("http://{host}:{port}");
    let address = resolve_readiness_address(host, port)?;
    let frontend = manager.run_script(
        frontend,
        "dev",
        [
            OsString::from("--host"),
            OsString::from(host),
            OsString::from("--port"),
            OsString::from(port.to_string()),
        ],
    );
    let mut args = vec![OsString::from("run")];
    args.extend(cargo_args);
    Ok(DevPlan {
        frontend,
        cargo: CommandSpec {
            program: "cargo".into(),
            args,
            current_dir: std::env::current_dir()?,
        },
        vite_url,
        readiness_address: address,
        readiness_timeout: Duration::from_secs(10),
    })
}
fn resolve_readiness_address(host: &str, port: u16) -> Result<SocketAddr, CliError> {
    let host = match host {
        "0.0.0.0" => "127.0.0.1",
        "::" => "::1",
        other => other,
    };
    Ok(SocketAddr::new(
        host.parse::<IpAddr>()
            .map_err(|_| CliError::Message(format!("invalid Vite host: {host}")))?,
        port,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_forwards_cargo_arguments_and_uses_loopback_for_wildcard_bind() {
        let plan = build_dev_plan(
            PathBuf::from("frontend"),
            PackageManager::Pnpm,
            "0.0.0.0",
            4173,
            vec![OsString::from("--bin"), OsString::from("server")],
        )
        .unwrap();
        assert_eq!(plan.vite_url, "http://0.0.0.0:4173");
        assert_eq!(plan.readiness_address, "127.0.0.1:4173".parse().unwrap());
        assert_eq!(
            plan.frontend.args,
            ["run", "dev", "--host", "0.0.0.0", "--port", "4173"].map(OsString::from)
        );
        assert_eq!(
            plan.cargo.args,
            ["run", "--bin", "server"].map(OsString::from)
        );
    }
}
