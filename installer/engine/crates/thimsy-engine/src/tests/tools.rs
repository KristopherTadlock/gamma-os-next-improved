use crate::events::{Event, MemorySink, Reporter};
use crate::tools::{self, Invocation};
use crate::Code;
use std::sync::Arc;

#[test]
fn invocation_display_quotes_spaces() {
    let inv = Invocation {
        tool: "spd_dump".into(),
        args: vec!["a".into(), "b c".into()],
        stdin: vec![],
    };
    assert_eq!(inv.display(), "spd_dump a \"b c\"");
}

#[test]
fn missing_tool_is_tool_missing() {
    let err = tools::locate("definitely-not-a-thimsy-tool").unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ToolMissing);
}

#[cfg(unix)]
#[test]
fn runs_a_tool_feeding_stdin_and_reading_lines() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("thimsy-echo-tool");
    std::fs::write(&tool, "#!/bin/sh\nread a\necho \"got:$a\"\necho to-stderr >&2\n[ \"$1\" = fail ] && exit 3\nexit 0\n").unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Point PATH at the dir via a fully qualified search: locate() checks PATH last.
    let old = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![dir.path().to_path_buf()];
    paths.extend(std::env::split_paths(&old));
    // SAFETY: tests in this binary don't read PATH concurrently except locate().
    unsafe { std::env::set_var("PATH", std::env::join_paths(paths).unwrap()) };
    let sink = Arc::new(MemorySink::default());
    let rep = Reporter::new(sink.clone());
    let mut lines = Vec::new();
    let inv = Invocation {
        tool: "thimsy-echo-tool".into(),
        args: vec![],
        stdin: vec!["yes".into()],
    };
    tools::run(&inv, dir.path(), &rep, |l| lines.push(l.to_string())).unwrap();
    // Both streams reach the callback (spd_dump logs progress on stderr).
    lines.sort();
    assert_eq!(lines, ["got:yes", "to-stderr"]);
    assert!(sink
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, Event::Log { message, .. } if message == "to-stderr")));
    let inv = Invocation {
        tool: "thimsy-echo-tool".into(),
        args: vec!["fail".into()],
        stdin: vec!["x".into()],
    };
    let err = tools::run(&inv, dir.path(), &rep, |_| {}).unwrap_err();
    assert_eq!(crate::error::code_of(&err).0, Code::ToolFailed);
    assert!(err.to_string().contains("to-stderr"));
}
