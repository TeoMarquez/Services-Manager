use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_workspace() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "services-manager-cli-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("migrations")).unwrap();
    fs::create_dir_all(root.join("mock")).unwrap();
    fs::write(
        root.join("migrations/01_initial.sql"),
        include_str!("../migrations/01_initial.sql"),
    )
    .unwrap();
    fs::write(
        root.join("migrations/02_add_present.sql"),
        include_str!("../migrations/02_add_present.sql"),
    )
    .unwrap();
    fs::write(
        root.join("migrations/03_discovery_checkpoint.sql"),
        include_str!("../migrations/03_discovery_checkpoint.sql"),
    )
    .unwrap();
    fs::write(
        root.join("migrations/04_operational_state.sql"),
        include_str!("../migrations/04_operational_state.sql"),
    )
    .unwrap();
    fs::write(
        root.join("migrations/05_manager_settings.sql"),
        include_str!("../migrations/05_manager_settings.sql"),
    )
    .unwrap();
    fs::write(
        root.join("mock/systemd.json"),
        include_str!("../mock/systemd.json"),
    )
    .unwrap();
    root
}

#[test]
fn cli_discovers_services_and_lists_them() {
    let workspace = test_workspace();
    let mut child = Command::new(env!("CARGO_BIN_EXE_Services-Manager"))
        .current_dir(&workspace)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"12\nminecraft\n2\n0\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "status: {}\nstdout: {}\nstderr: {}",
        output.status,
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("telegram-bot.service"),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(stdout.contains("minecraft.service"), "{stdout}");
    assert!(stdout.contains("hermes.service"), "{stdout}");
    assert!(stdout.contains("present:true"), "{stdout}");
    assert!(stdout.contains("1 result(s)"), "{stdout}");

    fs::remove_dir_all(workspace).unwrap();
}
