use crate::error::CliError;
use command_group::GroupChild;
use std::{
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};
pub fn wait(
    child: &mut GroupChild,
    address: SocketAddr,
    timeout: Duration,
) -> Result<(), CliError> {
    let started = Instant::now();
    loop {
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return Ok(());
        }
        if let Some(status) = child.try_wait()? {
            return Err(CliError::Message(format!(
                "frontend exited before Vite became ready at {address}: {status}"
            )));
        }
        if started.elapsed() >= timeout {
            return Err(CliError::Message(format!(
                "Vite did not become ready at {address} within {} seconds",
                timeout.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
