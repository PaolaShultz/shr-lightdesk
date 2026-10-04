use std::{
    io::Write,
    process::{Command, Stdio},
};
fn run(script: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_shr-lightdesk"))
        .arg("simulate")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}
#[test]
fn manual_operator_can_build_store_play_and_release() {
    let out = run(
        "select 11 12\nset INT 45\ncue record 1\ngo 1 1\nclear hold\nreturn playback INT\nconfirm\nstatus\nquit\n",
    );
    assert!(!out.contains("ERROR:"), "{out}");
    assert!(out.contains("Playback { slot: 0, cue: 1 }"));
    assert!(out.contains("45.0%"));
}
#[test]
fn fault_loop_recovers_lost_ack() {
    let out = run("autoack off\nmaster 50\ndropack\nmaster 90\nreconnect\nstatus\nquit\n");
    assert!(out.contains("disconnected: edits disabled"));
    assert!(out.contains("Ready"));
    assert!(!out.contains("panicked"));
}
#[test]
fn malformed_input_is_recoverable_and_no_hardware_command_exists() {
    let out = run(
        "select 11\nset INT NaN\nset PAN 40\nblackout nonsense\ngo 1 0\nmidi audio-sim 153 36 100\ndmx arm\nstatus\nquit\n",
    );
    assert!(out.contains("invalid value"));
    assert!(out.contains("Rejected"));
    assert!(out.contains("unknown command"));
    assert!(out.contains("physical UNKNOWN"));
}
