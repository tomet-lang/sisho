//! A directory of fake `tomet` and `twrit` programs on a `PATH` the test
//! controls, so no real tool is needed and every outcome can be produced on
//! demand. Shared by the tests of the commands that call tools.
#![allow(dead_code)] // each test file uses some of this, none uses all of it

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A directory holding fake tools, removed when dropped.
pub struct Sandbox {
    pub dir: PathBuf,
}

impl Sandbox {
    pub fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sisho-app-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    /// Install a fake tool called `name`. It prints `name 9.9.9` for
    /// `--version` (another version if `FAKE_VERSION_<NAME>` is set; an error,
    /// as a tool from before `--version` gave, if `FAKE_NO_VERSION` lists
    /// `name`), exits with `N` for `exit-N`, echoes the first line of its
    /// standard input for `read-stdin`, fails with a message when `FAKE_FAIL`
    /// lists `name:<first argument>`, and otherwise echoes its arguments.
    pub fn tool(self, name: &str) -> Self {
        let upper = name.to_uppercase();
        let script = format!(
            "#!/bin/sh\n\
             case \"$1\" in --version)\n\
               case \",$FAKE_NO_VERSION,\" in *\",{name},\"*) echo \"error: unexpected argument '--version' found\" >&2; exit 2;; esac\n\
               echo \"{name} ${{FAKE_VERSION_{upper}:-9.9.9}}\"; exit 0;; esac\n\
             case \"$1\" in exit-*) exit \"${{1#exit-}}\";; esac\n\
             case \"$1\" in read-stdin) read line; echo \"stdin: $line\"; exit 0;; esac\n\
             case \",$FAKE_FAIL,\" in *\",{name}:$1,\"*) echo \"boom from {name} $*\" >&2; exit 1;; esac\n\
             echo \"args: $*\"\n"
        );
        let path = self.dir.join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        self
    }

    pub fn both_tools(self) -> Self {
        self.tool("tomet").tool("twrit")
    }

    /// Run `sisho` with only this directory on `PATH`.
    pub fn sisho(&self, args: &[&str], fail: &str) -> Output {
        self.sisho_with(args, fail, &[])
    }

    /// [`Sandbox::sisho`], with more variables in the environment.
    pub fn sisho_with(&self, args: &[&str], fail: &str, env: &[(&str, &str)]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sisho"))
            .args(args)
            .current_dir(&self.dir)
            .env_clear()
            .env("PATH", &self.dir)
            .env("FAKE_FAIL", fail)
            .envs(env.iter().copied())
            .output()
            .unwrap()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn code(output: &Output) -> i32 {
    output.status.code().unwrap()
}
