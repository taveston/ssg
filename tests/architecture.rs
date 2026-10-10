//! Architecture checks (`docs/spec/architecture.md`, ARCH-01 to ARCH-34)
//! as text scans of the source tree, so a rule broken later fails the
//! build. Standard library only: no parsing crate, just enough reading of
//! Rust to find imports, functions and comments.
//!
//! The scans are deliberately rough. Where they would need real parsing to
//! be exact they err towards failing, so a false alarm means "look at
//! this", never "this is fine".

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// A function longer than this, outside content data and tests, fails
/// ARCH-21. The spec's guide is about 50 lines; its check allows 60.
const MAX_FN_LINES: usize = 60;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `.rs` file under `src/`, as (path relative to `src/` with forward
/// slashes, contents).
fn sources() -> BTreeMap<String, String> {
    fn walk(dir: &Path, base: &Path, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).expect("readable src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, base, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let relative = path.strip_prefix(base).unwrap().to_string_lossy();
                let text = fs::read_to_string(&path).expect("readable source");
                out.insert(relative.replace('\\', "/"), text);
            }
        }
    }
    let src = root().join("src");
    let mut out = BTreeMap::new();
    walk(&src, &src, &mut out);
    out
}

/// The part of a file before its unit tests.
fn non_test(text: &str) -> &str {
    text.split("#[cfg(test)]").next().unwrap_or(text)
}

/// The code of a line, without any `//` comment.
fn code_of(line: &str) -> &str {
    line.split("//").next().unwrap_or(line)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `word` appears in `text` as a whole word.
fn has_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
    })
}

/// ARCH-01: the three domain folders, each with a plugin in its `mod.rs`,
/// and a `main.rs` with no function but `main`.
#[test]
fn arch_01_domain_folders_each_have_a_plugin() {
    let sources = sources();
    for domain in ["sim", "content", "view"] {
        let module = sources
            .get(&format!("{domain}/mod.rs"))
            .unwrap_or_else(|| panic!("src/{domain}/mod.rs is missing"));
        assert!(
            module.contains("impl Plugin for"),
            "src/{domain}/mod.rs has no plugin"
        );
    }
    let main = &sources["main.rs"];
    for line in main.lines().map(code_of) {
        if let Some(rest) = line.trim_start().strip_prefix("fn ") {
            assert!(
                rest.starts_with("main("),
                "main.rs defines a function other than main: {line}"
            );
        }
    }
}

/// ARCH-02: simulation and content never reach into the view, or into
/// Bevy's rendering, UI, picking, gizmo, camera or window types.
#[test]
fn arch_02_sim_and_content_do_not_depend_on_the_view() {
    const FORBIDDEN: [&str; 8] = [
        "crate::view",
        "bevy::ui",
        "bevy::picking",
        "bevy::render",
        "bevy::window",
        "Gizmos",
        "Camera",
        "Window",
    ];
    for (path, text) in sources() {
        if !(path.starts_with("sim/") || path.starts_with("content/")) {
            continue;
        }
        for (number, line) in text.lines().enumerate() {
            let code = code_of(line);
            for word in FORBIDDEN {
                assert!(
                    !has_word(code, word),
                    "src/{path}:{}: uses {word}",
                    number + 1
                );
            }
        }
    }
}

/// The view module a file is, as a path inside `view/` (`markers`,
/// `camera::input`, or `camera` for `camera/mod.rs`).
fn view_module(path: &str) -> Option<String> {
    let inside = path.strip_prefix("view/")?.strip_suffix(".rs")?;
    let module = inside.strip_suffix("/mod").unwrap_or(inside);
    (module != "mod").then(|| module.replace('/', "::"))
}

/// The view modules a file imports, outside its tests. `super::x` inside a
/// submodule folder (`camera/`) means a sibling in that folder.
fn view_imports(module: &str, text: &str) -> BTreeSet<String> {
    let folder = module.rsplit_once("::").map(|(parent, _)| parent);
    let mut deps = BTreeSet::new();
    for line in non_test(text).lines().map(code_of) {
        for (prefix, sibling) in [("crate::view::", false), ("super::", true)] {
            for (at, _) in line.match_indices(prefix) {
                let rest = &line[at + prefix.len()..];
                let first: String = rest.chars().take_while(|&c| is_word_char(c)).collect();
                let second: String = rest[first.len()..]
                    .strip_prefix("::")
                    .map(|r| r.chars().take_while(|&c| is_word_char(c)).collect())
                    .unwrap_or_default();
                let dep = match (sibling, folder) {
                    (true, Some(folder)) => format!("{folder}::{first}"),
                    _ if first == "camera" && !second.is_empty() => format!("camera::{second}"),
                    _ => first,
                };
                if !dep.is_empty() && dep != module {
                    deps.insert(dep);
                }
            }
        }
    }
    deps
}

/// ARCH-03: no import cycles between view modules.
#[test]
fn arch_03_view_modules_import_each_other_without_cycles() {
    let graph: HashMap<String, BTreeSet<String>> = sources()
        .iter()
        .filter_map(|(path, text)| {
            let module = view_module(path)?;
            let deps = view_imports(&module, text);
            Some((module, deps))
        })
        .collect();
    assert!(graph.len() > 10, "found the view modules");

    // Depth-first search for a back edge, reporting the cycle it closes.
    fn visit(
        node: &str,
        graph: &HashMap<String, BTreeSet<String>>,
        state: &mut HashMap<String, bool>,
        stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        match state.get(node) {
            Some(true) => {
                let start = stack.iter().position(|n| n == node).unwrap();
                let mut cycle = stack[start..].to_vec();
                cycle.push(node.to_string());
                return Some(cycle);
            }
            Some(false) => return None,
            None => {}
        }
        state.insert(node.to_string(), true);
        stack.push(node.to_string());
        for dep in graph.get(node).into_iter().flatten() {
            if graph.contains_key(dep)
                && let Some(cycle) = visit(dep, graph, state, stack)
            {
                return Some(cycle);
            }
        }
        stack.pop();
        state.insert(node.to_string(), false);
        None
    }
    let mut state = HashMap::new();
    for node in graph.keys() {
        if let Some(cycle) = visit(node, &graph, &mut state, &mut Vec::new()) {
            panic!("view import cycle: {}", cycle.join(" -> "));
        }
    }
}

/// ARCH-12: the render scale lives only in `view/scale.rs`, and neither the
/// simulation nor content converts to render units.
#[test]
fn arch_12_render_scale_only_in_view_scale() {
    for (path, text) in sources() {
        let code: String = text.lines().map(code_of).collect::<Vec<_>>().join("\n");
        if path != "view/scale.rs" {
            assert!(
                !code.contains("RENDER_UNITS_PER"),
                "src/{path} uses the render scale constant"
            );
        }
        if path.starts_with("sim/") || path.starts_with("content/") {
            assert!(
                !code.contains("km_to_render"),
                "src/{path} converts to render units"
            );
        }
    }
}

/// Each top-level or `impl`-level function in non-test code, with its
/// length in lines, signature included.
fn function_lengths(text: &str) -> Vec<(String, usize)> {
    let lines: Vec<&str> = non_test(text).lines().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        let after_vis = trimmed
            .strip_prefix("pub(crate) ")
            .or_else(|| trimmed.strip_prefix("pub(super) "))
            .or_else(|| trimmed.strip_prefix("pub "))
            .unwrap_or(trimmed);
        if let Some(rest) = after_vis.strip_prefix("fn ")
            && indent <= 4
        {
            let name: String = rest.chars().take_while(|&c| is_word_char(c)).collect();
            let close = format!("{}}}", " ".repeat(indent));
            let end = (i..lines.len())
                .find(|&j| lines[j] == close || (j == i && line.trim_end().ends_with('}')))
                .unwrap_or(lines.len() - 1);
            found.push((name, end - i + 1));
            i = end;
        }
        i += 1;
    }
    found
}

/// ARCH-21: systems and helpers stay small. Content data is exempt, as are
/// tests.
#[test]
fn arch_21_functions_stay_small() {
    let mut too_long = Vec::new();
    for (path, text) in sources() {
        if path.starts_with("content/") || path == "headless_tests.rs" {
            continue;
        }
        for (name, length) in function_lengths(&text) {
            if length > MAX_FN_LINES {
                too_long.push(format!("src/{path}: {name} is {length} lines"));
            }
        }
    }
    assert!(too_long.is_empty(), "too long:\n{}", too_long.join("\n"));
}

/// The IDs `docs/spec/system-view.md` marks as built: neither struck
/// through, withdrawn nor "not built".
fn built_system_view_ids() -> Vec<String> {
    let spec = fs::read_to_string(root().join("docs/spec/system-view.md")).expect("spec");
    spec.lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // | ID | Requirement | Since |
            if cells.len() != 5 {
                return None;
            }
            let (id, requirement, since) = (cells[1], cells[2], cells[3]);
            let is_id = id.split_once('-').is_some_and(|(prefix, number)| {
                prefix.chars().all(|c| c.is_ascii_uppercase())
                    && !number.is_empty()
                    && number.chars().all(|c| c.is_ascii_digit())
            });
            let withdrawn = requirement.contains("Withdrawn") || id.starts_with("~~");
            let built = !since.contains("not built");
            (is_id && !withdrawn && built).then(|| id.to_string())
        })
        .collect()
}

/// ARCH-23: every built system-view ID is cited in a source comment.
#[test]
fn arch_23_every_built_spec_id_is_cited_in_a_comment() {
    let ids = built_system_view_ids();
    assert!(
        ids.len() > 30,
        "read the spec's ID table ({} IDs)",
        ids.len()
    );
    let comments: String = sources()
        .values()
        .flat_map(|text| text.lines())
        .filter_map(|line| line.split_once("//").map(|(_, comment)| comment))
        .collect::<Vec<_>>()
        .join("\n");
    let missing: Vec<&String> = ids.iter().filter(|id| !has_word(&comments, id)).collect();
    assert!(
        missing.is_empty(),
        "built IDs no comment cites: {missing:?}"
    );
}

/// ARCH-33: `allow(dead_code)` only on tunables waiting for an unbuilt
/// feature, each naming the ID it waits for.
#[test]
fn arch_33_dead_code_allowed_only_on_waiting_tunables() {
    for (path, text) in sources() {
        let lines: Vec<&str> = text.lines().collect();
        for (number, line) in lines.iter().enumerate() {
            if !code_of(line).contains("allow(dead_code)") {
                continue;
            }
            assert_eq!(
                path,
                "view/tunables.rs",
                "src/{path}:{}: dead code allowed outside the tunables",
                number + 1
            );
            let doc: String = lines[..number]
                .iter()
                .rev()
                .take_while(|l| l.trim_start().starts_with("///"))
                .copied()
                .collect();
            let names_an_id = doc.split_whitespace().any(|word| {
                word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-')
                    .split_once('-')
                    .is_some_and(|(p, n)| {
                        p.len() >= 2
                            && p.chars().all(|c| c.is_ascii_uppercase())
                            && n.chars().all(|c| c.is_ascii_digit())
                    })
            });
            assert!(
                doc.contains("Waits for") && names_an_id,
                "src/{path}:{}: a waiting tunable must say which ID it waits for",
                number + 1
            );
        }
    }
}

/// ARCH-34: every module starts with a `//!` header that names the spec IDs
/// it implements.
#[test]
fn arch_34_every_module_has_a_header_naming_its_ids() {
    for (path, text) in sources() {
        assert!(text.starts_with("//!"), "src/{path} has no //! header");
        let header: String = text
            .lines()
            .take_while(|line| line.starts_with("//!"))
            .collect::<Vec<_>>()
            .join(" ");
        let names_an_id = header
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .any(|word| {
                word.split_once('-').is_some_and(|(prefix, number)| {
                    prefix.len() >= 3
                        && prefix.chars().all(|c| c.is_ascii_uppercase())
                        && number.len() == 2
                        && number.chars().all(|c| c.is_ascii_digit())
                })
            });
        assert!(names_an_id, "src/{path}'s header names no spec ID");
    }
}

/// The scanner itself: imports in a camera submodule resolve to camera
/// siblings, and function lengths count the signature and body.
#[test]
fn the_scanner_reads_imports_and_lengths() {
    let deps = view_imports(
        "camera::input",
        "use super::state::X;\nuse crate::view::fold::Y;\nuse crate::view::camera::zoom::Z;",
    );
    let expected: BTreeSet<String> = ["camera::state", "fold", "camera::zoom"]
        .map(String::from)
        .into();
    assert_eq!(deps, expected);
    assert_eq!(view_module("view/camera/mod.rs").as_deref(), Some("camera"));
    assert_eq!(view_module("view/mod.rs"), None);
    let lengths = function_lengths("fn a() {\n    x();\n}\n\npub fn b(\n    y: u8,\n) {\n}\n");
    assert_eq!(lengths, vec![("a".to_string(), 3), ("b".to_string(), 4)]);
}
