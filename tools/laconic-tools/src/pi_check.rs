// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! `check-pi`: smoke-tests the Pi or OMP package over the agent's RPC mode, without a model
//! request. CI runs it after installing Pi from `ci/pi`.

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread;
use std::time::{Duration, Instant};

const RPC_TIMEOUT: Duration = Duration::from_secs(30);
const SHARED_MANIFEST_FIELDS: [&str; 5] = ["name", "version", "description", "license", "homepage"];

/// A Pi extension that adds `/reload-probe` and, once the flag file exists, records 'normal mode'
/// as handled so the check can see that disabled Laconic passes ordinary input on.
const RELOAD_PROBE: &str = r#"import { existsSync } from "node:fs";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
const passthroughProbeFlag = __PASSTHROUGH_PROBE_FLAG__;
export default function (pi: ExtensionAPI) {
  pi.registerCommand("reload-probe", {
    description: "Reload Pi for smoke testing",
    handler: async (_args, ctx) => { await ctx.reload(); },
  });
  pi.on("input", async (event) => {
    if (!existsSync(passthroughProbeFlag) || event.text.trim().toLowerCase() !== "normal mode") {
      return { action: "continue" };
    }
    pi.appendEntry("passthrough-probe", { seen: true });
    return { action: "handled" };
  });
}
"#;

macro_rules! ensure {
    ($cond:expr, $($message:tt)+) => {
        if !$cond {
            return Err(format!($($message)+));
        }
    };
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{} is not JSON: {e}", path.display()))
}

/// `package.json` and `kimi.plugin.json` agree, and the Pi and OMP entries point at the extension.
fn validate_package_manifest(root: &Path) -> Result<(), String> {
    let package = read_json(&root.join("package.json"))?;
    let kimi = read_json(&root.join("kimi.plugin.json"))?;
    for field in SHARED_MANIFEST_FIELDS {
        ensure!(
            package.get(field) == kimi.get(field),
            "package.json and kimi.plugin.json disagree on {field}"
        );
    }
    let pi = json!({"extensions": ["./extensions/laconic.ts"], "skills": ["./skills"]});
    ensure!(
        package.get("pi") == Some(&pi),
        "package.json's pi entry is not {pi}"
    );
    let omp = json!({"extensions": ["./extensions/laconic.ts"]});
    ensure!(
        package.get("omp") == Some(&omp),
        "package.json's omp entry is not {omp}"
    );
    Ok(())
}

/// The first `name` on `PATH`, as a shell would find it. On Windows only names with an
/// executable extension match, as in cmd.exe and Python's `shutil.which`: an extensionless
/// npm shim is a shell script that Windows cannot start.
pub(crate) fn which(name: &str) -> Option<PathBuf> {
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| {
            extensions
                .iter()
                .map(move |ext| dir.join(format!("{name}{ext}")))
        })
        .find(|candidate| candidate.is_file())
}

/// `vars`, this process's environment, without model credentials and pointed at a throwaway
/// agent directory.
fn isolated_env(
    vars: impl IntoIterator<Item = (String, String)>,
    agent_dir: &Path,
) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = vars
        .into_iter()
        .filter(|(key, _)| {
            !key.ends_with("_API_KEY")
                && key != "ANTHROPIC_AUTH_TOKEN"
                && key != "OPENAI_ACCESS_TOKEN"
        })
        .collect();
    env.push((
        "PI_CODING_AGENT_DIR".into(),
        agent_dir.display().to_string(),
    ));
    env.push(("PI_SKIP_VERSION_CHECK".into(), "1".into()));
    env.push(("PI_TELEMETRY".into(), "0".into()));
    env
}

/// A directory under the system temporary directory, removed when dropped. Like Python's
/// `tempfile.mkdtemp`, creation fails on an existing path and the directory is private on Unix.
pub(crate) struct TempDir(pub(crate) PathBuf);

impl TempDir {
    pub(crate) fn new(prefix: &str) -> Result<Self, String> {
        // Tests create directories in parallel, so the clock alone could repeat a name.
        static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let path =
            std::env::temp_dir().join(format!("{prefix}{}-{nanos}-{count}", std::process::id()));
        #[cfg(unix)]
        let created =
            std::os::unix::fs::DirBuilderExt::mode(&mut fs::DirBuilder::new(), 0o700).create(&path);
        #[cfg(not(unix))]
        let created = fs::create_dir(&path);
        created.map_err(|e| format!("cannot create {}: {e}", path.display()))?;
        Ok(Self(path))
    }

    /// Removes the directory, reporting the failure that `Drop` would ignore.
    fn remove(&self) -> Result<(), String> {
        fs::remove_dir_all(&self.0).map_err(|e| format!("cannot remove {}: {e}", self.0.display()))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// An agent in `--mode rpc`: one JSON command per stdin line, one JSON event per stdout line.
struct Rpc {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    stderr: PathBuf,
}

impl Rpc {
    fn start(
        executable: &Path,
        root: &Path,
        env: &[(String, String)],
        stderr: PathBuf,
        args: &[String],
    ) -> Result<Self, String> {
        let log = fs::File::create(&stderr)
            .map_err(|e| format!("cannot create {}: {e}", stderr.display()))?;
        let mut child = Command::new(executable)
            .args(["--mode", "rpc"])
            .args(args)
            .current_dir(root)
            .env_clear()
            .envs(env.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log)
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", executable.display()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Agent RPC stdout is unavailable")?;
        let (sender, lines) = channel();
        // Dropping the sender at end of output is how `next` learns the agent exited.
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let stdin = child.stdin.take();
        Ok(Self {
            child,
            stdin,
            lines,
            stderr,
        })
    }

    fn stderr(&self) -> String {
        fs::read_to_string(&self.stderr).unwrap_or_default()
    }

    /// The next event; `waiting_for` completes "did not ... within 30 seconds".
    fn next(&self, waiting_for: &str, before: &str) -> Result<Value, String> {
        match self.lines.recv_timeout(RPC_TIMEOUT) {
            Ok(line) => serde_json::from_str(&line)
                .map_err(|e| format!("Agent RPC wrote a line that is not JSON ({e}): {line}")),
            Err(RecvTimeoutError::Timeout) => Err(format!(
                "Agent RPC did not {waiting_for} within {} seconds",
                RPC_TIMEOUT.as_secs()
            )),
            Err(RecvTimeoutError::Disconnected) => Err(format!(
                "Agent RPC exited before {before}: {}",
                self.stderr()
            )),
        }
    }

    /// Sends `command` with `id` and returns its response and every event up to it.
    fn request(&mut self, id: &str, command: Value) -> Result<(Value, Vec<Value>), String> {
        let mut payload = command;
        payload["id"] = json!(id);
        let stdin = self
            .stdin
            .as_mut()
            .ok_or("Agent RPC stdin is unavailable")?;
        writeln!(stdin, "{payload}")
            .and_then(|()| stdin.flush())
            .map_err(|e| format!("cannot write to the agent RPC: {e}"))?;
        let mut events = Vec::new();
        loop {
            let event = self.next("respond", "responding")?;
            let done = event["type"] == "response" && event["id"] == id;
            events.push(event);
            if done {
                let response = events.last().cloned().unwrap_or_default();
                return Ok((response, events));
            }
        }
    }

    /// Every event up to and including the first that matches `wanted`.
    fn events_until(&self, wanted: impl Fn(&Value) -> bool) -> Result<Vec<Value>, String> {
        let mut events = Vec::new();
        loop {
            let event = self.next("emit the expected event", "emitting the expected event")?;
            let done = wanted(&event);
            events.push(event);
            if done {
                return Ok(events);
            }
        }
    }

    /// Closes stdin and waits 5 seconds for the agent to exit; then, as the Python check did,
    /// stops it and waits 5 more. An agent that is still running is killed and fails the check.
    fn close(self) -> Result<(), String> {
        self.close_within(Duration::from_secs(5))
    }

    fn close_within(mut self, grace: Duration) -> Result<(), String> {
        drop(self.stdin.take());
        let status = match self.wait_for(grace)? {
            Some(status) => status,
            None => {
                terminate(&mut self.child)?;
                let Some(status) = self.wait_for(grace)? else {
                    self.child
                        .kill()
                        .map_err(|e| format!("cannot kill the agent RPC: {e}"))?;
                    self.child
                        .wait()
                        .map_err(|e| format!("cannot wait for the agent RPC: {e}"))?;
                    return Err(format!(
                        "Agent RPC ignored the end of its input and SIGTERM, so it was killed: {}",
                        self.stderr()
                    ));
                };
                status
            }
        };
        ensure!(
            status.success() || terminated(&status),
            "Agent RPC exited with {status}: {}",
            self.stderr()
        );
        Ok(())
    }

    fn wait_for(&mut self, grace: Duration) -> Result<Option<ExitStatus>, String> {
        let deadline = Instant::now() + grace;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return Ok(Some(status)),
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
                Ok(None) => return Ok(None),
                Err(e) => return Err(format!("cannot wait for the agent RPC: {e}")),
            }
        }
    }
}

impl Drop for Rpc {
    /// For early returns: an agent that is still running is killed and reaped.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Sends SIGTERM, as Python's `terminate()` did; std can only send SIGKILL.
#[cfg(unix)]
fn terminate(child: &mut Child) -> Result<(), String> {
    unsafe extern "C" {
        // kill(2) from the C library std links; pid_t is i32 on every Unix Rust supports.
        safe fn kill(pid: i32, signal: i32) -> i32;
    }
    const SIGTERM: i32 = 15;
    let pid = i32::try_from(child.id()).map_err(|e| format!("agent RPC pid: {e}"))?;
    ensure!(
        kill(pid, SIGTERM) == 0,
        "cannot send SIGTERM to the agent RPC: {}",
        std::io::Error::last_os_error()
    );
    Ok(())
}

/// Python's `terminate()` is TerminateProcess here: the agent exits 1, which fails the check.
#[cfg(not(unix))]
fn terminate(child: &mut Child) -> Result<(), String> {
    child
        .kill()
        .map_err(|e| format!("cannot stop the agent RPC: {e}"))
}

#[cfg(unix)]
fn terminated(status: &ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    status.signal() == Some(15)
}

#[cfg(not(unix))]
fn terminated(_: &ExitStatus) -> bool {
    false
}

/// Runs `body`, then closes the agent even when `body` failed, reporting both failures.
fn with_agent(
    mut rpc: Rpc,
    body: impl FnOnce(&mut Rpc) -> Result<(), String>,
) -> Result<(), String> {
    let result = body(&mut rpc);
    match (result, rpc.close()) {
        (Ok(()), closed) => closed,
        (Err(e), Ok(())) => Err(e),
        (Err(e), Err(closed)) => Err(format!("{e}\nThen, closing the agent: {closed}")),
    }
}

/// The status texts the extension set under the `laconic` key; `None` clears the status.
fn status_texts(events: &[Value]) -> Vec<Option<&Value>> {
    events
        .iter()
        .filter(|e| {
            e["type"] == "extension_ui_request"
                && e["method"] == "setStatus"
                && e["statusKey"] == "laconic"
        })
        .map(|e| e.get("statusText").filter(|text| !text.is_null()))
        .collect()
}

fn shows_on(events: &[Value]) -> bool {
    status_texts(events).iter().any(|text| {
        text.and_then(Value::as_str)
            .is_some_and(|t| t.contains("LACONIC ON"))
    })
}

fn clears_status(events: &[Value]) -> bool {
    status_texts(events).contains(&None)
}

fn entry_list(response: &Value) -> Result<&Vec<Value>, String> {
    response["data"]["entries"]
        .as_array()
        .ok_or_else(|| format!("get_entries returned no entries: {response}"))
}

fn message_count(response: &Value, custom_type: &str) -> Result<usize, String> {
    Ok(entry_list(response)?
        .iter()
        .filter(|e| e["type"] == "custom_message" && e["customType"] == custom_type)
        .count())
}

/// The mode the extension persisted last.
fn latest_enabled(response: &Value) -> Result<bool, String> {
    entry_list(response)?
        .iter()
        .rfind(|e| e["type"] == "custom" && e["customType"] == "laconic-state")
        .and_then(|e| e["data"]["enabled"].as_bool())
        .ok_or_else(|| "No persisted laconic state found".to_owned())
}

/// Every command's name; a list that is missing or holds a command without one is an error.
fn command_names(commands: &Value) -> Result<Vec<&str>, String> {
    commands
        .as_array()
        .ok_or_else(|| format!("Agent RPC sent no command list: {commands}"))?
        .iter()
        .map(|c| {
            c["name"]
                .as_str()
                .ok_or_else(|| format!("Agent RPC listed a command without a name: {c}"))
        })
        .collect()
}

fn prompt(message: &str) -> Value {
    json!({"type": "prompt", "message": message})
}

fn succeeded(response: &Value, step: &str) -> Result<(), String> {
    ensure!(
        response["success"] == true,
        "{step} did not succeed: {response}"
    );
    Ok(())
}

/// The Pi session: startup, reload, toggles, the skill alias, the stop phrase and passthrough.
fn pi_session(rpc: &mut Rpc, passthrough_flag: &Path) -> Result<(), String> {
    let (commands, events) = rpc.request("commands", json!({"type": "get_commands"}))?;
    let names = command_names(&commands["data"]["commands"])?;
    ensure!(
        names.contains(&"laconic"),
        "Pi lists no /laconic command: {names:?}"
    );
    ensure!(
        names.contains(&"skill:laconic"),
        "Pi lists no /skill:laconic command: {names:?}"
    );
    ensure!(shows_on(&events), "LACONIC ON was not shown at startup");

    let (entries, _) = rpc.request("entries-startup", json!({"type": "get_entries"}))?;
    ensure!(
        message_count(&entries, "laconic-rules")? == 1,
        "startup did not inject the rules once"
    );
    ensure!(
        message_count(&entries, "laconic-disabled")? == 0,
        "startup recorded a disabled message"
    );

    let (reloaded, events) = rpc.request("reload-enabled", prompt("/reload-probe"))?;
    succeeded(&reloaded, "/reload-probe")?;
    ensure!(shows_on(&events), "LACONIC ON was not shown after a reload");
    let (entries, _) = rpc.request("entries-reloaded", json!({"type": "get_entries"}))?;
    ensure!(
        message_count(&entries, "laconic-rules")? == 1,
        "Rules were injected twice for one active session"
    );

    let (toggled, events) = rpc.request("toggle-off", prompt("/laconic"))?;
    succeeded(&toggled, "/laconic (off)")?;
    ensure!(
        clears_status(&events),
        "turning Laconic off did not clear its status"
    );
    let (entries, _) = rpc.request("entries-toggled-off", json!({"type": "get_entries"}))?;
    ensure!(
        !latest_enabled(&entries)?,
        "turning Laconic off was not persisted"
    );
    ensure!(
        message_count(&entries, "laconic-disabled")? == 1,
        "turning Laconic off recorded no disabled message"
    );

    let (reloaded, events) = rpc.request("reload-disabled", prompt("/reload-probe"))?;
    succeeded(&reloaded, "/reload-probe (disabled)")?;
    ensure!(
        !shows_on(&events),
        "LACONIC ON was shown after a reload whilst off"
    );
    let (entries, _) = rpc.request("entries-reload-disabled", json!({"type": "get_entries"}))?;
    ensure!(
        message_count(&entries, "laconic-rules")? == 1,
        "a reload whilst off injected the rules"
    );
    ensure!(
        message_count(&entries, "laconic-disabled")? == 1,
        "a reload whilst off recorded another disabled message"
    );

    let (toggled, events) = rpc.request("toggle-on", prompt("/laconic"))?;
    succeeded(&toggled, "/laconic (on)")?;
    ensure!(
        shows_on(&events),
        "LACONIC ON was not shown after turning Laconic on"
    );
    let (entries, _) = rpc.request("entries-toggled-on", json!({"type": "get_entries"}))?;
    ensure!(
        latest_enabled(&entries)?,
        "turning Laconic on was not persisted"
    );
    ensure!(
        message_count(&entries, "laconic-rules")? == 2,
        "turning Laconic on did not inject the rules"
    );

    let (off, _) = rpc.request("explicit-off", prompt("/laconic off"))?;
    succeeded(&off, "/laconic off")?;

    let (skill, events) = rpc.request("skill-alias", prompt("/skill:laconic"))?;
    succeeded(&skill, "/skill:laconic")?;
    ensure!(
        !events.iter().any(|e| e["type"] == "agent_start"),
        "/skill:laconic started a model turn"
    );
    let (entries, _) = rpc.request("entries-enabled", json!({"type": "get_entries"}))?;
    ensure!(
        latest_enabled(&entries)?,
        "/skill:laconic did not turn Laconic on"
    );
    ensure!(
        message_count(&entries, "laconic-rules")? == 3,
        "/skill:laconic did not inject the rules"
    );

    let (stopped, events) = rpc.request("stop-phrase", prompt("normal mode"))?;
    succeeded(&stopped, "the stop phrase")?;
    ensure!(
        clears_status(&events),
        "the stop phrase did not clear the status"
    );
    let (entries, _) = rpc.request("entries-stopped", json!({"type": "get_entries"}))?;
    ensure!(
        !latest_enabled(&entries)?,
        "the stop phrase was not persisted"
    );

    fs::write(passthrough_flag, "")
        .map_err(|e| format!("cannot create {}: {e}", passthrough_flag.display()))?;
    let (passthrough, _) = rpc.request("disabled-passthrough", prompt("normal mode"))?;
    succeeded(&passthrough, "ordinary input whilst off")?;
    let (entries, _) = rpc.request("entries-passthrough", json!({"type": "get_entries"}))?;
    ensure!(
        entry_list(&entries)?.iter().any(|e| {
            e["type"] == "custom"
                && e["customType"] == "passthrough-probe"
                && e["data"]["seen"] == true
        }),
        "Disabled mode swallowed ordinary input"
    );
    Ok(())
}

/// OMP has no `get_commands` or entries over RPC here: check the commands and status it announces.
fn omp_session(rpc: &mut Rpc) -> Result<(), String> {
    let events = rpc.events_until(|e| e["type"] == "available_commands_update")?;
    let last = events.last().cloned().unwrap_or_default();
    let names = command_names(&last["commands"])?;
    ensure!(
        names.contains(&"laconic"),
        "OMP lists no /laconic command: {names:?}"
    );
    ensure!(
        names.contains(&"skill:laconic"),
        "OMP lists no /skill:laconic command: {names:?}"
    );
    ensure!(shows_on(&events), "LACONIC ON was not shown at startup");
    Ok(())
}

pub fn run(root: &Path, runtime: &str) -> Result<String, String> {
    ensure!(
        matches!(runtime, "pi" | "omp"),
        "--runtime must be pi or omp, not {runtime}"
    );
    validate_package_manifest(root)?;
    let executable =
        which(runtime).ok_or_else(|| format!("{runtime} executable is not available"))?;
    let work = TempDir::new(&format!("laconic-{runtime}-"))?;
    let result = smoke_test(root, runtime, &executable, &work.0);
    match (result, work.remove()) {
        (Ok(passed), removed) => removed.map(|()| passed),
        (Err(e), Ok(())) => Err(e),
        (Err(e), Err(removed)) => Err(format!("{e}\nThen, {removed}")),
    }
}

/// The smoke test proper, in the throwaway directory `work`.
fn smoke_test(
    root: &Path,
    runtime: &str,
    executable: &Path,
    work: &Path,
) -> Result<String, String> {
    let agent_dir = work.join("agent");
    fs::create_dir(&agent_dir)
        .map_err(|e| format!("cannot create {}: {e}", agent_dir.display()))?;
    let mut env = isolated_env(std::env::vars(), &agent_dir);
    if runtime == "omp" {
        env.retain(|(key, _)| key != "PI_CODING_AGENT_DIR");
    } else {
        let install = Command::new(executable)
            .arg("install")
            .arg(root)
            .current_dir(root)
            .env_clear()
            .envs(env.iter().map(|(k, v)| (k, v)))
            .output()
            .map_err(|e| format!("cannot run {} install: {e}", executable.display()))?;
        ensure!(
            install.status.success(),
            "{runtime} install failed with {}: {}",
            install.status,
            String::from_utf8_lossy(&install.stderr)
        );
    }
    let mut extension_args: Vec<String> = Vec::new();
    if runtime == "omp" {
        extension_args = vec![
            "--no-extensions".into(),
            "-e".into(),
            root.display().to_string(),
        ];
    }

    let reload_probe = agent_dir.join("reload-probe.ts");
    let passthrough_flag = agent_dir.join("passthrough-probe-enabled");
    let flag = serde_json::to_string(&passthrough_flag.display().to_string())
        .map_err(|e| e.to_string())?;
    fs::write(
        &reload_probe,
        RELOAD_PROBE.replace("__PASSTHROUGH_PROBE_FLAG__", &flag),
    )
    .map_err(|e| format!("cannot write {}: {e}", reload_probe.display()))?;

    let mut args = vec!["--no-session".to_owned()];
    args.extend(extension_args.iter().cloned());
    args.extend([
        "-e".to_owned(),
        reload_probe.display().to_string(),
        "--laconic".to_owned(),
    ]);
    let rpc = Rpc::start(executable, root, &env, work.join("session.stderr"), &args)?;
    if runtime == "omp" {
        with_agent(rpc, omp_session)?;
        return Ok("omp extension smoke test passed".into());
    }
    with_agent(rpc, |rpc| pi_session(rpc, &passthrough_flag))?;

    fs::write(agent_dir.join(".laconic-always"), "")
        .map_err(|e| format!("cannot create .laconic-always: {e}"))?;
    let mut args = vec!["--no-session".to_owned()];
    args.extend(extension_args);
    let rpc = Rpc::start(executable, root, &env, work.join("always-on.stderr"), &args)?;
    with_agent(rpc, |rpc| {
        let (_, events) = rpc.request("always-on", json!({"type": "get_state"}))?;
        ensure!(
            shows_on(&events),
            "LACONIC ON was not shown with .laconic-always"
        );
        Ok(())
    })?;
    Ok("pi extension smoke test passed".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(text: Value) -> Value {
        json!({"type": "extension_ui_request", "method": "setStatus", "statusKey": "laconic", "statusText": text})
    }

    #[test]
    fn the_repository_manifests_agree() {
        assert_eq!(validate_package_manifest(&crate::root()), Ok(()));
    }

    #[test]
    fn status_reads_only_laconic_and_treats_null_as_cleared() {
        let events = [
            status(json!("LACONIC ON (full)")),
            json!({"type": "extension_ui_request", "method": "setStatus", "statusKey": "other", "statusText": "LACONIC ON"}),
            status(Value::Null),
        ];
        assert!(shows_on(&events));
        assert!(clears_status(&events));
        assert!(!shows_on(&events[1..]));
        assert!(!clears_status(&events[..2]));
        let missing = [
            json!({"type": "extension_ui_request", "method": "setStatus", "statusKey": "laconic"}),
        ];
        assert!(clears_status(&missing));
    }

    #[test]
    fn entries_are_counted_by_type_and_the_last_state_wins() {
        let response = json!({"data": {"entries": [
            {"type": "custom_message", "customType": "laconic-rules"},
            {"type": "custom", "customType": "laconic-state", "data": {"enabled": true}},
            {"type": "custom_message", "customType": "laconic-rules"},
            {"type": "custom", "customType": "laconic-state", "data": {"enabled": false}},
        ]}});
        assert_eq!(message_count(&response, "laconic-rules"), Ok(2));
        assert_eq!(message_count(&response, "laconic-disabled"), Ok(0));
        assert_eq!(latest_enabled(&response), Ok(false));
        let unset = json!({"data": {"entries": [{"type": "custom", "customType": "laconic-state", "data": {}}]}});
        assert!(latest_enabled(&unset).is_err());
        assert!(message_count(&json!({"data": {}}), "laconic-rules").is_err());
    }

    #[test]
    fn the_isolated_environment_drops_model_credentials() {
        let vars = [
            ("LACONIC_TEST_API_KEY", "x"),
            ("ANTHROPIC_AUTH_TOKEN", "x"),
            ("OPENAI_ACCESS_TOKEN", "x"),
            ("HOME", "/home/user"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()));
        let env = isolated_env(vars, Path::new("/tmp/agent"));
        assert_eq!(
            env,
            [
                ("HOME", "/home/user"),
                ("PI_CODING_AGENT_DIR", "/tmp/agent"),
                ("PI_SKIP_VERSION_CHECK", "1"),
                ("PI_TELEMETRY", "0"),
            ]
            .map(|(k, v)| (k.to_string(), v.to_string()))
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_work_directory_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let work = TempDir::new("laconic-test-").unwrap();
        let mode = fs::metadata(&work.0).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    #[test]
    fn a_command_without_a_name_is_an_error() {
        let commands = json!([{"name": "laconic"}, {"name": "skill:laconic"}]);
        assert_eq!(
            command_names(&commands).unwrap(),
            ["laconic", "skill:laconic"]
        );
        let malformed = json!([{"name": "laconic"}, {"name": "skill:laconic"}, {}]);
        assert!(command_names(&malformed).is_err());
        assert!(command_names(&Value::Null).is_err());
    }

    #[test]
    fn removing_the_work_directory_reports_failure() {
        let work = TempDir::new("laconic-test-").unwrap();
        fs::write(work.0.join("file"), "x").unwrap();
        work.remove().unwrap();
        assert!(!work.0.exists());
        assert!(work.remove().unwrap_err().starts_with("cannot remove"));
    }

    /// An agent script for `Rpc::start`, which passes `--mode rpc` that `sh` ignores here.
    #[cfg(unix)]
    fn agent(work: &TempDir, script: &str) -> Rpc {
        use std::os::unix::fs::PermissionsExt;
        let path = work.0.join("agent.sh");
        fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        Rpc::start(&path, &work.0, &[], work.0.join("stderr"), &[]).unwrap()
    }

    /// One test, so no other test forks whilst a script is written and run ('text file busy').
    #[cfg(unix)]
    #[test]
    fn closing_accepts_end_of_input_or_sigterm_and_fails_an_agent_that_ignores_both() {
        let work = TempDir::new("laconic-test-").unwrap();
        let grace = Duration::from_millis(300);
        agent(&work, "exec /bin/cat > /dev/null")
            .close_within(grace)
            .unwrap();
        agent(&work, "exec /bin/sleep 30")
            .close_within(grace)
            .unwrap();
        let error = agent(&work, "exit 3").close_within(grace).unwrap_err();
        assert!(error.starts_with("Agent RPC exited with"), "{error}");
        let error = agent(&work, "trap '' TERM\nexec /bin/sleep 30")
            .close_within(grace)
            .unwrap_err();
        assert!(
            error.contains("ignored the end of its input and SIGTERM"),
            "{error}"
        );
    }
}
