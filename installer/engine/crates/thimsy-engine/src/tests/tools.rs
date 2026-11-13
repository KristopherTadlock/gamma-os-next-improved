use crate::error::{Code, EngineError};
use crate::events::{Event, MemorySink, Reporter};
use crate::tools::{self, Invocation};
use std::sync::Arc;

fn inv(tool: &str, args: &[&str], stdin: &[&str], prompt: Option<&str>) -> Invocation {
    Invocation {
        tool: tool.into(),
        args: args.iter().map(|s| s.to_string()).collect(),
        stdin: stdin.iter().map(|s| s.to_string()).collect(),
        answer_prompt: prompt.map(str::to_string),
    }
}

#[test]
fn invocation_display_quotes_spaces() {
    assert_eq!(
        inv("spd_dump", &["a", "b c"], &[], None).display(),
        "spd_dump a \"b c\""
    );
}

#[test]
fn missing_tool_is_tool_missing() {
    let err = tools::locate("definitely-not-a-thimsy-tool").unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ToolMissing);
}

#[cfg(unix)]
fn script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

#[cfg(unix)]
fn quiet() -> (Reporter, Arc<MemorySink>) {
    let sink = Arc::new(MemorySink::default());
    (Reporter::new(sink.clone()), sink)
}

#[cfg(unix)]
#[test]
fn runs_a_tool_feeding_stdin_and_reading_lines() {
    let dir = tempfile::tempdir().unwrap();
    script(
        dir.path(),
        "thimsy-echo-tool",
        "#!/bin/sh\nread a\necho \"got:$a\"\necho to-stderr >&2\n[ \"$1\" = fail ] && exit 3\nexit 0\n",
    );
    // Found through the tools folder (per-thread test hook; no env changes).
    tools::test::TOOLS_DIR.with(|d| *d.borrow_mut() = Some(dir.path().to_path_buf()));
    let (rep, sink) = quiet();
    let mut lines = Vec::new();
    // Without a prompt, answers are written up front as before.
    let i = inv("thimsy-echo-tool", &[], &["yes"], None);
    tools::run(&i, dir.path(), &rep, |l| lines.push(l.to_string())).unwrap();
    // Both streams reach the callback (spd_dump logs progress on stderr).
    lines.sort();
    assert_eq!(lines, ["got:yes", "to-stderr"]);
    assert!(sink
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, Event::Log { message, .. } if message == "to-stderr")));
    let i = inv("thimsy-echo-tool", &["fail"], &["x"], None);
    let err = tools::run(&i, dir.path(), &rep, |_| {}).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ToolFailed);
    assert!(err.to_string().contains("to-stderr"));
}

/// Asks like spd_dump (no newline after the question) up to three times,
/// after checking whether anything was written to stdin before it asked.
#[cfg(unix)]
const ASKS_THREE_TIMES: &str = r#"#!/usr/bin/env bash
if read -r -t 1 early; then echo "early:$early"; fi
for i in 1 2 3; do
  printf 'Answer "yes" to confirm the "write partition" command: '
  if read -r a; then echo "got$i:$a"; else echo "operation is not confirmed ($i)" >&2; exit 1; fi
done
"#;

#[cfg(unix)]
#[test]
fn answers_only_when_asked_and_closes_stdin_when_they_run_out() {
    let dir = tempfile::tempdir().unwrap();
    let exe = script(dir.path(), "asker", ASKS_THREE_TIMES);
    let (rep, _) = quiet();
    let mut lines = Vec::new();
    let mut asked = Vec::new();
    let i = inv(
        "asker",
        &[],
        &["yes", "yes"],
        Some(crate::flash::SPD_PROMPT),
    );
    let err = tools::run_exe(
        &exe,
        &i,
        dir.path(),
        &rep,
        |l| lines.push(l.to_string()),
        |n| {
            asked.push(n);
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ToolFailed);
    assert!(!lines.iter().any(|l| l.starts_with("early")), "{lines:?}");
    assert!(lines.contains(&"got1:yes".to_string()), "{lines:?}");
    assert!(lines.contains(&"got2:yes".to_string()), "{lines:?}");
    // The third question finds stdin closed: not confirmed, not answered.
    assert!(
        lines.iter().any(|l| l.contains("not confirmed (3)")),
        "{lines:?}"
    );
    assert_eq!(asked, [0, 1]);
}

#[cfg(unix)]
#[test]
fn a_refused_answer_closes_stdin_and_returns_the_reason() {
    let dir = tempfile::tempdir().unwrap();
    let exe = script(dir.path(), "asker", ASKS_THREE_TIMES);
    let (rep, _) = quiet();
    let mut lines = Vec::new();
    let i = inv(
        "asker",
        &[],
        &["yes", "yes", "yes"],
        Some(crate::flash::SPD_PROMPT),
    );
    let err = tools::run_exe(
        &exe,
        &i,
        dir.path(),
        &rep,
        |l| lines.push(l.to_string()),
        |_| Err(EngineError::new(Code::WrongDevice, "not this one").into()),
    )
    .unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::WrongDevice);
    assert!(!lines.iter().any(|l| l.starts_with("got")), "{lines:?}");
    assert!(
        lines.iter().any(|l| l.contains("not confirmed (1)")),
        "{lines:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_question_split_across_writes_is_still_seen() {
    let dir = tempfile::tempdir().unwrap();
    let exe = script(
        dir.path(),
        "slow-asker",
        "#!/usr/bin/env bash\nprintf 'Answer \"y'\nsleep 0.3\nprintf 'es\" to confirm the \"erase partition\" command: '\nread -r a || exit 1\necho \"got:$a\"\n",
    );
    let (rep, _) = quiet();
    let mut lines = Vec::new();
    let i = inv("slow-asker", &[], &["yes"], Some(crate::flash::SPD_PROMPT));
    tools::run_exe(
        &exe,
        &i,
        dir.path(),
        &rep,
        |l| lines.push(l.to_string()),
        |_| Ok(()),
    )
    .unwrap();
    assert!(lines.contains(&"got:yes".to_string()), "{lines:?}");
}

#[cfg(unix)]
#[test]
fn bytes_that_arent_utf8_dont_stop_the_output() {
    let dir = tempfile::tempdir().unwrap();
    // Lots of output after the bad bytes: if a reader stopped, the tool
    // would block on a full pipe or die of SIGPIPE.
    let exe = script(
        dir.path(),
        "noisy",
        "#!/bin/sh\nprintf '\\377\\376 bad bytes\\n' >&2\nseq 1 50000\nseq 1 50000 >&2\necho done\n",
    );
    let (rep, _) = quiet();
    let mut lines = Vec::new();
    let i = inv("noisy", &[], &[], None);
    tools::run_exe(
        &exe,
        &i,
        dir.path(),
        &rep,
        |l| lines.push(l.to_string()),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(lines.len(), 1 + 50_000 * 2 + 1);
    assert!(lines
        .iter()
        .any(|l| l.contains('\u{fffd}') && l.ends_with("bad bytes")));
    assert!(lines.contains(&"done".to_string()));
    assert_eq!(lines.iter().filter(|l| *l == "50000").count(), 2);
}
