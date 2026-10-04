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
    assert!(out.contains("attribute unavailable for complete selection"));
    assert!(out.contains("unknown command"));
    assert!(out.contains("physical UNKNOWN"));
}

#[test]
fn detached_drafts_and_blackout_reveal_require_confirmation() {
    let out = run(include_str!("fixtures/ld01-operator.txt"));
    assert!(!out.contains("ERROR:"), "{out}");
    assert!(out.contains("BlackoutOff"));
    assert!(out.contains("SIMULATOR draft"));
    assert!(out.contains("45.0%"));
}

#[test]
fn real_lux_file_mode_uses_decoder_and_visible_provenance() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/lx03/commands.json")).unwrap();
    let page = &corpus["cases"][8]["replies"][0];
    let bytes = serde_json::to_vec(page).unwrap();
    let path =
        std::env::temp_dir().join(format!("lightdesk-provider-{}.frames", std::process::id()));
    let mut framed = (bytes.len() as u32).to_be_bytes().to_vec();
    framed.extend(bytes);
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(&framed).unwrap();
    drop(file);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_shr-lightdesk"))
        .args([
            "--provider",
            "lux",
            "--snapshot-file",
            path.to_str().unwrap(),
            "--show-id",
            "11111111-1111-4111-8111-111111111111",
            "--epoch",
            "9",
        ])
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let out = String::from_utf8(output.stdout).unwrap();
    assert!(out.contains("LUX READ-ONLY"));
    assert!(out.contains("Hold 0"));
    assert!(out.contains("contributors"));
    assert!(out.contains("observed UNKNOWN"));
    assert!(!out.contains("SIMULATOR"));
}
