use crate::{
    ports::{PortEntry, find_port, list_ports},
    process::{force_kill_pid, is_pid_running, terminate_pid},
    tui,
};
use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

#[derive(Parser)]
#[command(version, about = "Find and free local ports from your terminal.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    List(ListArgs),
    Find {
        port: u16,
    },
    Kill {
        port: u16,
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Args)]
struct ListArgs {
    #[arg(long)]
    json: bool,
}

pub fn run() -> Result<()> {
    match Cli::parse().command {
        None => tui::run(),
        Some(Command::List(args)) => list(args),
        Some(Command::Find { port }) => find(port),
        Some(Command::Kill { port, force }) => kill(port, force),
    }
}

fn list(args: ListArgs) -> Result<()> {
    let entries = list_ports()?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    print!("{}", format_entries_table(&entries));
    Ok(())
}

fn find(port: u16) -> Result<()> {
    let Some(entry) = find_port(port)? else {
        println!("Port {port} is free.");
        return Ok(());
    };

    print!("{}", format_find_result(port, &entry));
    Ok(())
}

fn kill(port: u16, force: bool) -> Result<()> {
    let Some(entry) = find_port(port)? else {
        println!("Port {port} is free.");
        return Ok(());
    };

    if !force
        && !confirm(&format!(
            "Kill {} PID {} using port {}? [y/N] ",
            entry.process_name, entry.pid, entry.port
        ))?
    {
        println!("Cancelled.");
        return Ok(());
    }

    if let Err(error) = terminate_pid(entry.pid) {
        bail!("{error}\n\nTry running with sudo or terminate it manually.");
    }

    println!("Sent SIGTERM to {} PID {}.", entry.process_name, entry.pid);
    thread::sleep(Duration::from_millis(300));
    if is_pid_running(entry.pid)
        && confirm("Process is still running. Force kill with SIGKILL? [y/N] ")?
    {
        if let Err(error) = force_kill_pid(entry.pid) {
            bail!("{error}\n\nTry running with sudo or terminate it manually.");
        }
        println!("Sent SIGKILL to {} PID {}.", entry.process_name, entry.pid);
    }
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
}

fn format_entries_table(entries: &[PortEntry]) -> String {
    let mut output = format!(
        "{:<7} {:<9} {:<8} {:<16} {:<18} COMMAND\n",
        "PORT", "PROTOCOL", "PID", "PROCESS", "ADDRESS"
    );
    for entry in entries {
        output.push_str(&format!(
            "{:<7} {:<9} {:<8} {:<16} {:<18} {}\n",
            entry.port,
            entry.protocol,
            entry.pid,
            entry.process_name,
            entry.address,
            entry.command.as_deref().unwrap_or_default()
        ));
    }
    output
}

fn format_find_result(port: u16, entry: &PortEntry) -> String {
    format!(
        "Port {port} is used by:\n\nProcess: {}\nPID:     {}\nCommand: {}\n",
        entry.process_name,
        entry.pid,
        entry.command.as_deref().unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::Protocol;

    fn entry() -> PortEntry {
        PortEntry {
            port: 3000,
            protocol: Protocol::Tcp,
            address: "127.0.0.1".to_string(),
            pid: 18422,
            process_name: "node".to_string(),
            command: Some("next dev".to_string()),
        }
    }

    #[test]
    fn formats_table_output() {
        let output = format_entries_table(&[entry()]);

        assert!(output.contains("PORT"));
        assert!(output.contains("3000"));
        assert!(output.contains("node"));
        assert!(output.contains("next dev"));
    }

    #[test]
    fn formats_find_output() {
        let output = format_find_result(3000, &entry());

        assert_eq!(
            output,
            "Port 3000 is used by:\n\nProcess: node\nPID:     18422\nCommand: next dev\n"
        );
    }
}
