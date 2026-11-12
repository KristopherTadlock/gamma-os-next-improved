//! Translations of the text the engine shows (stage titles, prompts, hints,
//! device guides, profile descriptions). Like Thimsy's own `i18n/<code>.json`,
//! tables are keyed by the English text, so anything untranslated stays
//! English. Codes match Thimsy's: ar da de el es fi fr he it ja ko nb nl pl pt
//! ru sv th tr zh-cn zh-tw.

use std::cell::RefCell;
use std::collections::HashMap;

pub const LANGUAGES: &[&str] = &[
    "en", "ar", "da", "de", "el", "es", "fi", "fr", "he", "it", "ja", "ko", "nb", "nl", "pl", "pt",
    "ru", "sv", "th", "tr", "zh-cn", "zh-tw",
];

thread_local! {
    // Per thread: the CLI uses one thread; tests run in parallel.
    static TABLE: RefCell<Option<HashMap<String, String>>> = const { RefCell::new(None) };
}

/// Normalises codes like `zh_CN`, `zh-Hans`, `pt-BR`, `no` to Thimsy's.
pub fn normalise(code: &str) -> String {
    let c = code.trim().replace('_', "-").to_ascii_lowercase();
    match c.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "zh-sg" => "zh-cn".into(),
        "zh-tw" | "zh-hant" | "zh-hk" | "zh-mo" => "zh-tw".into(),
        "no" | "nb" | "nn" | "nb-no" => "nb".into(),
        "iw" => "he".into(),
        _ => c.split('-').next().unwrap_or("en").to_string(),
    }
}

/// Activates a language from the catalog's `i18n/<code>.yaml` files.
pub fn set_language(code: &str, files: &[(String, String)]) -> anyhow::Result<()> {
    let code = normalise(code);
    if code == "en" {
        TABLE.with(|t| *t.borrow_mut() = None);
        return Ok(());
    }
    if !LANGUAGES.contains(&code.as_str()) {
        anyhow::bail!(
            "unsupported language {code}; supported: {}",
            LANGUAGES.join(" ")
        );
    }
    let name = format!("i18n/{code}.yaml");
    let table = match files.iter().find(|(n, _)| *n == name) {
        Some((_, text)) => serde_yaml::from_str::<HashMap<String, String>>(text)?,
        None => HashMap::new(),
    };
    TABLE.with(|t| *t.borrow_mut() = Some(table));
    Ok(())
}

/// Translates `s` if the active language has it.
pub fn tr(s: &str) -> String {
    TABLE.with(|t| {
        t.borrow()
            .as_ref()
            .and_then(|t| t.get(s).cloned())
            .unwrap_or_else(|| s.to_string())
    })
}

/// Every English string the engine can show: catalog guide text, profile
/// descriptions, and the literals passed to `tr`, `stage`, `prompt` and
/// `hint` in the engine sources under `src_dir`. Written to
/// `catalog/i18n/en.yaml` by `thimsy-installer dev i18n-strings`.
pub fn source_strings(
    cat: &crate::catalog::Catalog,
    src_dir: &std::path::Path,
) -> std::io::Result<Vec<String>> {
    let mut set = std::collections::BTreeSet::new();
    for p in &cat.profiles {
        set.insert(p.name.clone());
        set.insert(p.summary.clone());
    }
    for d in &cat.devices {
        set.extend(d.notes.iter().cloned());
        set.extend(d.blocks.iter().cloned());
        set.extend(d.host_message.iter().cloned());
        set.extend(d.update_requires.iter().cloned());
        let g = &d.guides;
        for s in g
            .backup
            .iter()
            .chain(&g.unlock.steps)
            .chain(&g.download_mode)
            .chain(&g.first_boot)
        {
            set.insert(s.title.clone());
            set.insert(s.body.clone());
        }
    }
    let mut files = Vec::new();
    fn walk(d: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
        for e in std::fs::read_dir(d)? {
            let p = e?.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "tests") {
                    walk(&p, out)?;
                }
            } else if p.extension().is_some_and(|x| x == "rs")
                && p.file_name().is_some_and(|n| n != "testutil.rs")
            {
                out.push(p);
            }
        }
        Ok(())
    }
    walk(src_dir, &mut files)?;
    files.sort();
    for f in files {
        let text = std::fs::read_to_string(&f)?;
        let text = text.split("#[cfg(test)]").next().unwrap_or("");
        // (call prefix, index of the first translatable literal argument)
        for (prefix, skip) in [
            ("tr(", 0),
            (".stage(", 1),
            (".prompt(", 1),
            (".hint(", 0),
            ("unwrap_or(", 0),
            ("map_or(", 0),
        ] {
            let mut rest = text;
            while let Some(i) = rest.find(prefix) {
                rest = &rest[i + prefix.len()..];
                let lits = leading_literals(rest, 3);
                let titles = skip == 1; // stage/prompt titles may be one word
                for s in lits.into_iter().skip(skip) {
                    if (titles || s.contains(' ')) && !s.is_empty() && !s.contains("{}") {
                        set.insert(s);
                    }
                }
            }
        }
    }
    Ok(set.into_iter().collect())
}

/// Up to `n` consecutive string-literal arguments at the start of `s`.
fn leading_literals(s: &str, n: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while out.len() < n {
        while chars.peek().is_some_and(|c| c.is_whitespace() || *c == ',') {
            chars.next();
        }
        if chars.peek() != Some(&'"') {
            break;
        }
        chars.next();
        let mut lit = String::new();
        let mut closed = false;
        while let Some(c) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some('n') => lit.push('\n'),
                    Some('t') => lit.push('\t'),
                    Some(o) => lit.push(o),
                    None => break,
                },
                '"' => {
                    closed = true;
                    break;
                }
                c => lit.push(c),
            }
        }
        if !closed {
            break;
        }
        out.push(lit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_list_is_up_to_date_and_every_language_matches_it() {
        let cat = crate::catalog::Catalog::bundled().unwrap();
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let want = source_strings(&cat, &src).unwrap();
        let en_text = &cat
            .files
            .iter()
            .find(|(n, _)| n == "i18n/en.yaml")
            .expect("i18n/en.yaml")
            .1;
        let en: std::collections::BTreeMap<String, String> = serde_yaml::from_str(en_text).unwrap();
        let have: Vec<String> = en.keys().cloned().collect();
        assert_eq!(
            have, want,
            "catalog/i18n/en.yaml is stale: run thimsy-installer dev i18n-strings"
        );
        fn placeholders(s: &str) -> Vec<String> {
            let mut v: Vec<String> = s
                .match_indices('{')
                .filter_map(|(i, _)| s[i..].find('}').map(|j| s[i..i + j + 1].to_string()))
                .collect();
            v.sort();
            v
        }
        for code in LANGUAGES.iter().filter(|c| **c != "en") {
            let name = format!("i18n/{code}.yaml");
            let text = &cat
                .files
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .1;
            let t: std::collections::BTreeMap<String, String> = serde_yaml::from_str(text).unwrap();
            let keys: Vec<String> = t.keys().cloned().collect();
            assert_eq!(keys, want, "{name} keys differ from en.yaml");
            for (k, v) in &t {
                assert!(!v.trim().is_empty(), "{name}: empty translation for {k:?}");
                assert_eq!(
                    placeholders(k),
                    placeholders(v),
                    "{name}: placeholders changed in {k:?}"
                );
            }
        }
    }

    #[test]
    fn normalises_codes() {
        assert_eq!(normalise("zh_Hans"), "zh-cn");
        assert_eq!(normalise("zh-TW"), "zh-tw");
        assert_eq!(normalise("pt-BR"), "pt");
        assert_eq!(normalise("no"), "nb");
        assert_eq!(LANGUAGES.len(), 22);
    }
}
