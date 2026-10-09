//! `gray-openwiki` — sidecar + CLI for the openwiki plugin.
//!
//! `/wiki` generates a structured Markdown wiki under `.gray/wiki/`
//! (index, architecture, modules/, flows/, glossary); `/wiki update`
//! refreshes only the sections affected by changed source files, using a
//! mtime+size ledger at `.gray/wiki/.ledger.json`. The model-facing
//! surface is one tool (`wiki_status`) and a `prompt/context` hook that
//! mentions the wiki when present.
//!
//! Wire: NDJSON on stdio — see ~/grayplugins/PORTING.md. Bare argv (no
//! subcommand) runs the sidecar loop; `manifest` prints the install
//! manifest; `init|update|status|commit|scan` are the CLI verbs.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};

/// `prompt/context` / `command/run` / `tool/call` param session cwd.
fn session_cwd(params: &Value) -> PathBuf {
    let hint = params
        .pointer("/session/cwd")
        .and_then(Value::as_str)
        .or_else(|| params.get("cwd").and_then(Value::as_str))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().ok().unwrap_or_else(|| PathBuf::from(".")));
    gray_openwiki::resolve_root(&hint)
}

fn tool_call(params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    if name != gray_openwiki::TOOL_NAME && !name.is_empty() {
        return json!({ "content": format!("unknown tool: {name}"), "is_error": true });
    }
    let path = params
        .get("args")
        .and_then(|a| a.get("path"))
        .and_then(Value::as_str);
    let root = gray_openwiki::root_for(path, params);
    json!({ "content": gray_openwiki::status(&root).to_string() })
}

fn command_run(params: &Value) -> Value {
    let argv: Vec<&str> = params
        .get("argv")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let root = session_cwd(params);
    gray_openwiki::run_wiki(&root, &argv)
}

/// One request → `Some(reply)`, or `None` for notifications. The bool asks
/// the loop to exit after writing the reply.
fn handle(req: &Value) -> (Option<Value>, bool) {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let Some(id) = id else {
        // Notifications need no reply; shutdown exits the loop.
        return (None, method == "plugin/shutdown");
    };
    let result = match method {
        "plugin/manifest" => gray_openwiki::manifest(),
        "prompt/context" => {
            let root = session_cwd(&params);
            json!({ "text": gray_openwiki::context_text(&root).unwrap_or_default() })
        }
        "tool/call" => tool_call(&params),
        "command/run" => command_run(&params),
        "plugin/shutdown" => return (Some(json!({ "id": id, "result": {} })), true),
        _ => {
            let error = json!({ "code": -32601, "message": "method not found" });
            return (Some(json!({ "id": id, "error": error })), false);
        }
    };
    (Some(json!({ "id": id, "result": result })), false)
}

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let (reply, exit) = handle(&req);
        if let Some(reply) = reply {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
        if exit {
            break;
        }
    }
    Ok(())
}

fn cli_root(args: &[String]) -> PathBuf {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--root" {
            if let Some(p) = it.next() {
                return gray_openwiki::resolve_root(&PathBuf::from(p));
            }
        } else if !a.starts_with('-') {
            return gray_openwiki::resolve_root(&PathBuf::from(a));
        }
    }
    gray_openwiki::resolve_root(
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    )
}

const USAGE: &str = "gray-openwiki — codebase wiki plugin for gray

usage:
  gray-openwiki                  run the sidecar wire loop (stdin/stdout NDJSON)
  gray-openwiki sidecar          same, explicitly
  gray-openwiki manifest         print the install manifest (JSON)
  gray-openwiki init   [path]    stage a full wiki generation, print the prompt
  gray-openwiki update [path]    stage a scoped refresh, print the prompt
  gray-openwiki status [path]    print wiki status (JSON)
  gray-openwiki commit [--root p]  promote .ledger.pending.json to .ledger.json
  gray-openwiki scan   <path>    print the tracked-file snapshot (JSON)
";

fn cli(argv: &[String]) -> i32 {
    match argv.first().map(String::as_str) {
        None | Some("sidecar") => {
            if let Err(e) = sidecar() {
                eprintln!("gray-openwiki: sidecar: {e}");
                return 1;
            }
            0
        }
        Some("manifest") => {
            println!("{}", gray_openwiki::manifest());
            0
        }
        Some("init") | Some("update") => {
            let root = cli_root(&argv[1..]);
            let r = gray_openwiki::run_wiki(&root, &[argv[0].as_str()]);
            if let Some(p) = r.get("prompt").and_then(Value::as_str) {
                println!("{p}");
            } else {
                println!("{}", r["text"].as_str().unwrap_or("(no result)"));
            }
            0
        }
        Some("status") => {
            let root = cli_root(&argv[1..]);
            println!(
                "{}",
                serde_json::to_string_pretty(&gray_openwiki::status(&root)).unwrap()
            );
            0
        }
        Some("scan") => {
            let root = cli_root(&argv[1..]);
            let s = gray_openwiki::scan(&root);
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "files": s.files.len(),
                    "truncated": s.truncated,
                    "top_dirs": s.top_dirs,
                }))
                .unwrap()
            );
            0
        }
        Some("commit") => {
            let root = cli_root(&argv[1..]);
            match gray_openwiki::commit(&root) {
                Ok(msg) => {
                    println!("{msg}");
                    0
                }
                Err(e) => {
                    eprintln!("gray-openwiki: {e}");
                    1
                }
            }
        }
        Some("help") | Some("--help") | Some("-h") => {
            print!("{USAGE}");
            0
        }
        Some(other) => {
            eprintln!("gray-openwiki: unknown subcommand {other:?}\n\n{USAGE}");
            2
        }
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(cli(&argv));
}
