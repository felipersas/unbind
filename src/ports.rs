use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortEntry {
    pub port: u16,
    pub protocol: Protocol,
    pub address: String,
    pub pid: u32,
    pub process_name: String,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Protocol {
    #[serde(rename = "TCP")]
    Tcp,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "TCP"),
        }
    }
}

pub fn list_ports() -> Result<Vec<PortEntry>> {
    let output = Command::new("lsof")
        .args(["-n", "-P", "-iTCP", "-sTCP:LISTEN", "-F", "pcPn"])
        .output()
        .context("Could not find lsof.\n\nInstall it with your system package manager:\n- macOS: usually preinstalled\n- Arch: sudo pacman -S lsof\n- Ubuntu/Debian: sudo apt install lsof")?;

    if !output.status.success() && output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Could not inspect listening ports.{}\nTry running with sudo if you need full process details.",
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!("\nReason: {}", stderr.trim())
            }
        );
    }

    let stdout = String::from_utf8(output.stdout).context("lsof returned invalid UTF-8")?;
    let mut entries = parse_lsof(&stdout);
    fill_commands(&mut entries);
    Ok(entries)
}

pub fn parse_lsof(output: &str) -> Vec<PortEntry> {
    if output.lines().any(|line| {
        line.strip_prefix('p').is_some_and(|value| {
            value
                .chars()
                .next()
                .is_some_and(|char| char.is_ascii_digit())
        })
    }) {
        return parse_lsof_fields(output);
    }

    parse_lsof_table(output)
}

fn parse_lsof_fields(output: &str) -> Vec<PortEntry> {
    let mut entries = Vec::new();
    let mut current_pid = None;
    let mut current_process = None;
    let mut current_protocol = Protocol::Tcp;

    for line in output.lines().filter(|line| !line.is_empty()) {
        let Some((field, value)) = line.split_at_checked(1) else {
            continue;
        };
        match field {
            "p" => current_pid = value.parse().ok(),
            "c" => current_process = Some(value.to_string()),
            "P" => {
                current_protocol = match value {
                    "TCP" => Protocol::Tcp,
                    _ => continue,
                };
            }
            "n" => {
                let (Some(pid), Some(process_name), Some((address, port))) =
                    (current_pid, current_process.clone(), parse_endpoint(value))
                else {
                    continue;
                };
                entries.push(PortEntry {
                    port,
                    protocol: current_protocol,
                    address,
                    pid,
                    process_name: process_name.clone(),
                    command: Some(process_name),
                });
            }
            _ => {}
        }
    }

    dedupe(entries)
}

fn parse_lsof_table(output: &str) -> Vec<PortEntry> {
    let mut seen = HashSet::new();
    output
        .lines()
        .skip(1)
        .filter_map(parse_lsof_line)
        .filter(|entry| {
            seen.insert((
                entry.port,
                entry.address.clone(),
                entry.pid,
                entry.process_name.clone(),
            ))
        })
        .collect()
}

fn parse_lsof_line(line: &str) -> Option<PortEntry> {
    let columns: Vec<&str> = line.split_whitespace().collect();
    if columns.len() < 9 || columns.get(7) != Some(&"TCP") {
        return None;
    }

    let tcp_index = columns.iter().position(|column| *column == "TCP")?;
    let process_name = columns.first()?.to_string();
    let pid = columns.get(1)?.parse().ok()?;
    let endpoint = columns.get(tcp_index + 1)?;
    let (address, port) = parse_endpoint(endpoint)?;

    Some(PortEntry {
        port,
        protocol: Protocol::Tcp,
        address,
        pid,
        process_name: process_name.clone(),
        command: Some(process_name),
    })
}

fn parse_endpoint(endpoint: &str) -> Option<(String, u16)> {
    let endpoint = endpoint.strip_suffix(" (LISTEN)").unwrap_or(endpoint);
    let endpoint = endpoint.trim_end_matches("->").trim_start_matches('[');
    let (address, port) = endpoint.rsplit_once(':')?;
    let address = address.trim_start_matches('[').trim_end_matches(']');
    let port = port.trim_end_matches(']').parse().ok()?;
    Some((address.to_string(), port))
}

fn dedupe(entries: Vec<PortEntry>) -> Vec<PortEntry> {
    let mut seen = HashSet::new();
    entries
        .into_iter()
        .filter(|entry| {
            seen.insert((
                entry.port,
                entry.address.clone(),
                entry.pid,
                entry.process_name.clone(),
            ))
        })
        .collect()
}

pub fn find_port(port: u16) -> Result<Option<PortEntry>> {
    Ok(list_ports()?.into_iter().find(|entry| entry.port == port))
}

fn fill_commands(entries: &mut [PortEntry]) {
    let mut commands = HashMap::new();
    for entry in entries.iter() {
        commands.entry(entry.pid).or_insert_with(|| {
            process_command(entry.pid).unwrap_or_else(|| entry.process_name.clone())
        });
    }
    for entry in entries {
        entry.command = commands.get(&entry.pid).cloned();
    }
}

fn process_command(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let command = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!command.is_empty()).then_some(command)
}
