//! Wire-level tests: drive the built sidecar binary over stdio like the
//! gray host does (numeric host→sidecar ids, plugin/shutdown notify-exit).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

struct Harness {
    child: Child,
    stdin: ChildStdin,
    lines: std::sync::mpsc::Receiver<String>,
    _reader: std::thread::JoinHandle<()>,
}

impl Harness {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_gray-openwiki"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn gray-openwiki sidecar");
        let stdin = child.stdin.take().expect("stdin");
        let stdout: ChildStdout = child.stdout.take().expect("stdout");
        let (tx, rx) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            lines: rx,
            _reader: reader,
        }
    }

    fn send(&mut self, v: &serde_json::Value) {
        writeln!(self.stdin, "{v}").expect("write");
        self.stdin.flush().expect("flush");
    }

    fn next(&self) -> serde_json::Value {
        let line = self
            .lines
            .recv_timeout(Duration::from_secs(10))
            .expect("reply within 10s");
        serde_json::from_str(&line).expect("valid json reply")
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

trait WaitTimeout {
    fn wait_timeout(&mut self) -> bool;
}

impl WaitTimeout for Child {
    fn wait_timeout(&mut self) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            match self.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => return false,
            }
        }
        false
    }
}

#[test]
fn manifest_declares_wiki_surface() {
    let mut h = Harness::spawn();
    h.send(&serde_json::json!({"id": 1, "method": "plugin/manifest"}));
    let m = h.next();
    assert_eq!(m["id"], 1);
    assert_eq!(m["result"]["name"], "openwiki");
    assert_eq!(m["result"]["protocol"], "1.1");
    assert_eq!(m["result"]["sidecar_args"][0], "sidecar");
    assert_eq!(m["result"]["tools"][0]["name"], "wiki_status");
    assert_eq!(m["result"]["commands"][0], "/wiki");
}

#[test]
fn bare_wiki_returns_generation_prompt() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("x.rs"), b"fn x() {}").unwrap();
    let mut h = Harness::spawn();
    h.send(&serde_json::json!({
        "id": 1, "method": "command/run",
        "params": {"name": "/wiki", "argv": [],
                   "session": {"id": "s", "cwd": tmp.path().to_string_lossy()}}
    }));
    let r = h.next();
    assert_eq!(r["id"], 1);
    let prompt = r["result"]["prompt"].as_str().expect("prompt outcome");
    assert!(prompt.contains(".gray/wiki"));
    assert!(prompt.contains("commit"));
    // The run staged a pending ledger for the agent to commit later.
    assert!(tmp
        .path()
        .join(".gray/wiki/.ledger.pending.json")
        .exists());
}

#[test]
fn wiki_status_and_context_reflect_wiki_state() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tmp.path().to_string_lossy().to_string();
    let mut h = Harness::spawn();
    // No wiki yet: empty context, tool reports absent.
    h.send(&serde_json::json!({
        "id": 1, "method": "prompt/context",
        "params": {"session": {"id": "s", "cwd": cwd}}
    }));
    let r = h.next();
    assert_eq!(r["result"]["text"], "");
    h.send(&serde_json::json!({
        "id": 2, "method": "tool/call",
        "params": {"name": "wiki_status", "args": {"path": cwd}}
    }));
    let r = h.next();
    let status: serde_json::Value =
        serde_json::from_str(r["result"]["content"].as_str().unwrap()).unwrap();
    assert_eq!(status["present"], false);
    // Drop a wiki page in place: context hook now mentions it.
    std::fs::create_dir_all(tmp.path().join(".gray/wiki")).unwrap();
    std::fs::write(tmp.path().join(".gray/wiki/index.md"), b"# wiki").unwrap();
    h.send(&serde_json::json!({
        "id": 3, "method": "prompt/context",
        "params": {"session": {"id": "s", "cwd": cwd}}
    }));
    let r = h.next();
    assert!(r["result"]["text"].as_str().unwrap().contains(".gray/wiki"));
}

#[test]
fn notifications_silent_shutdown_exits() {
    let mut h = Harness::spawn();
    h.send(&serde_json::json!({"method": "event/notify", "params": {"type": "turn_end"}}));
    h.send(&serde_json::json!({"method": "plugin/shutdown", "params": {"reason": "test"}}));
    assert!(h.child.wait_timeout(), "sidecar exits on plugin/shutdown");
}
