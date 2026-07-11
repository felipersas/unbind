use anyhow::{Context, Result, anyhow};

pub fn terminate_pid(pid: u32) -> Result<()> {
    signal_pid(pid, nix::sys::signal::Signal::SIGTERM)
}

pub fn force_kill_pid(pid: u32) -> Result<()> {
    signal_pid(pid, nix::sys::signal::Signal::SIGKILL)
}

pub fn is_pid_running(pid: u32) -> bool {
    signal_pid(pid, None).is_ok()
}

fn signal_pid<S>(pid: u32, signal: S) -> Result<()>
where
    S: Into<Option<nix::sys::signal::Signal>>,
{
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(i32::try_from(pid).map_err(|_| anyhow!("invalid PID {pid}"))?),
        signal.into(),
    )
    .with_context(|| format!("Could not signal process PID {pid}"))?;
    Ok(())
}
