use super::{DevPlan, readiness};
use crate::{error::CliError, package_manager::CommandSpec};
use command_group::{CommandGroup, GroupChild};
use std::{
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

fn spawn(spec: &CommandSpec, vite_url: Option<&str>) -> Result<GroupChild, CliError> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.current_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(url) = vite_url {
        command.env("VITE_DEV_SERVER_URL", url);
    }
    command.group_spawn().map_err(|error| {
        CliError::Message(format!(
            "could not start {} in {}: {error}",
            spec.program.to_string_lossy(),
            spec.current_dir.display()
        ))
    })
}
fn terminate(child: &mut GroupChild) -> std::io::Result<()> {
    if child.try_wait()?.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}
pub fn run(plan: DevPlan, wait_for_ready: bool) -> Result<(), CliError> {
    let mut frontend = spawn(&plan.frontend, None)?;
    if wait_for_ready {
        if let Err(error) = readiness::wait(
            &mut frontend,
            plan.readiness_address,
            plan.readiness_timeout,
        ) {
            let _ = terminate(&mut frontend);
            return Err(error);
        }
    }
    let mut cargo = match spawn(&plan.cargo, Some(&plan.vite_url)) {
        Ok(child) => child,
        Err(error) => {
            let _ = terminate(&mut frontend);
            return Err(error);
        }
    };
    let shutdown = Arc::new(AtomicBool::new(false));
    let handler = Arc::clone(&shutdown);
    ctrlc::set_handler(move || handler.store(true, Ordering::SeqCst))
        .map_err(|error| CliError::Message(format!("could not install signal handler: {error}")))?;
    loop {
        if shutdown.load(Ordering::SeqCst) {
            terminate(&mut frontend)?;
            terminate(&mut cargo)?;
            return Ok(());
        }
        if let Some(status) = frontend.try_wait()? {
            let _ = terminate(&mut cargo);
            return if status.success() {
                Ok(())
            } else {
                Err(CliError::Message(format!("frontend exited with {status}")))
            };
        }
        if let Some(status) = cargo.try_wait()? {
            let _ = terminate(&mut frontend);
            return if status.success() {
                Ok(())
            } else {
                Err(CliError::Message(format!("cargo exited with {status}")))
            };
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
