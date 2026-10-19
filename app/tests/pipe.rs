//! A reader that closes the pipe early (`| head`) is not an error of `sisho`:
//! it ends quietly with the status a shell reports for a process killed by
//! `SIGPIPE`, and does not panic.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A vault whose link check prints far more than a pipe buffer holds, so the
/// output cannot all fit before the reader has closed.
fn vault_with_many_broken_links(name: &str) -> Dir {
    let dir = std::env::temp_dir().join(format!("sisho-app-pipe-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("cache")).unwrap();
    fs::create_dir_all(dir.join("vault")).unwrap();
    let mut text = String::from("@meta{}\n\n");
    for i in 0..4000 {
        text.push_str(&format!("@link(missing-{i}.txt)\n[ x ]\n\n"));
    }
    fs::write(dir.join("vault/a.tmt"), text).unwrap();
    Dir(dir)
}

#[test]
fn a_closed_pipe_ends_the_command_quietly_with_141() {
    let dir = vault_with_many_broken_links("links");
    let cache = dir.0.join("cache");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .arg("check-links")
        .current_dir(dir.0.join("vault"))
        .env("XDG_CACHE_HOME", &cache)
        .env("HOME", &cache)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The reader goes away without reading anything.
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();

    assert_eq!(output.status.code(), Some(141));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "nothing to say about a closed pipe: {stderr}"
    );
}

#[test]
fn a_closed_pipe_ends_list_quietly_with_141() {
    let dir = std::env::temp_dir().join(format!("sisho-app-pipe-list-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // Create many extensions so writing exceeds pipe buffer
    for i in 0..2000 {
        let path = dir.join(format!("sisho-ext{i}"));
        fs::write(&path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    let mut child = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .arg("--list")
        .env_clear()
        .env("PATH", &dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    let _ = fs::remove_dir_all(&dir);

    assert_eq!(output.status.code(), Some(141));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "nothing to say about a closed pipe: {stderr}"
    );
}
