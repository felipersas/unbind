use unbind::ports::{Protocol, parse_lsof};

#[test]
fn parses_lsof_field_output() {
    let entries = parse_lsof(include_str!("fixtures/lsof_fields_macos.txt"));

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].port, 3000);
    assert_eq!(entries[0].protocol, Protocol::Tcp);
    assert_eq!(entries[0].address, "127.0.0.1");
    assert_eq!(entries[0].pid, 18422);
    assert_eq!(entries[0].process_name, "node");
    assert_eq!(entries[1].port, 5432);
    assert_eq!(entries[1].address, "*");
    assert_eq!(entries[2].port, 51476);
}

#[test]
fn parses_lsof_field_ipv6_output() {
    let entries = parse_lsof(include_str!("fixtures/lsof_fields_ipv6.txt"));

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].address, "::1");
    assert_eq!(entries[0].port, 3000);
    assert_eq!(entries[1].process_name, "ControlCe");
    assert_eq!(entries[1].address, "*");
    assert_eq!(entries[1].port, 7000);
}

#[test]
fn parses_lsof_listening_tcp_rows() {
    let output = "\
COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
node    18422 user   22u  IPv4 123456      0t0  TCP 127.0.0.1:3000 (LISTEN)
postgres 902  user   11u  IPv4 987654      0t0  TCP *:5432 (LISTEN)
";

    let entries = parse_lsof(output);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].port, 3000);
    assert_eq!(entries[0].protocol, Protocol::Tcp);
    assert_eq!(entries[0].address, "127.0.0.1");
    assert_eq!(entries[0].pid, 18422);
    assert_eq!(entries[0].process_name, "node");
    assert_eq!(entries[1].port, 5432);
    assert_eq!(entries[1].address, "*");
}

#[test]
fn parses_ipv6_and_variable_lsof_columns() {
    let output = "\
COMMAND     PID USER   FD   TYPE             DEVICE SIZE/OFF NODE NAME
node      18422 user   22u  IPv6 0xae633716e6b7a305      0t0  TCP [::1]:3000 (LISTEN)
ControlCe   552 user   10u  IPv4 0x123456789abcdef0      0t0  TCP *:7000 (LISTEN)
";

    let entries = parse_lsof(output);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].address, "::1");
    assert_eq!(entries[0].port, 3000);
    assert_eq!(entries[1].process_name, "ControlCe");
    assert_eq!(entries[1].address, "*");
    assert_eq!(entries[1].port, 7000);
}

#[test]
fn deduplicates_identical_lsof_rows() {
    let output = "\
COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
rapportd  638 user   10u  IPv4 123456      0t0  TCP *:51475 (LISTEN)
rapportd  638 user   11u  IPv6 123457      0t0  TCP *:51475 (LISTEN)
";

    let entries = parse_lsof(output);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].port, 51475);
    assert_eq!(entries[0].pid, 638);
}
