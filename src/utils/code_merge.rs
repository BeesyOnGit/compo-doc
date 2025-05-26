use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::utils::utils::{convert_hash, read_from_file_ut};

pub fn get_imported_components(tsx: &str) -> Vec<String> {
    let import_re =
        Regex::new(r#"(?m)^\s*import\s+([^;]+?)\s+from\s+["']([^"']+)["'];?\s*$"#).unwrap();

    // reject obvious non-component assets
    let asset_re = Regex::new(r#"\.(svg|png|jpe?g|gif|webp|bmp|ico|ttf|woff2?)$"#).unwrap();

    // component name validators
    let re_ident = Regex::new(r#"[A-Za-z_][A-Za-z0-9_]*"#).unwrap();
    let re_upper = Regex::new(r#"^[A-Z][A-Za-z0-9_]*[a-z][A-Za-z0-9_]*$"#).unwrap();
    let reject_suffix = Regex::new(r#"(Store|Context|Provider|State|Atom|Slice)$"#).unwrap();

    let mut set = HashSet::new();

    for cap in import_re.captures_iter(tsx) {
        let clause = &cap[1];
        let spec = &cap[2];

        if asset_re.is_match(spec) {
            continue; // images / fonts / etc.
        }

        for m in re_ident.find_iter(clause) {
            let id = m.as_str();
            if re_upper.is_match(id) && !id.starts_with("use") && !reject_suffix.is_match(id) {
                set.insert(id.to_string());
            }
        }
    }

    let mut list: Vec<_> = set.into_iter().collect();
    list.sort();
    list
}

pub fn inline_components<'a>(tsx: &'a str, comp_map: &HashMap<&'a str, &'a str>) -> String {
    // Which external components are imported?
    let comps = get_imported_components(tsx); // ← from the earlier helper
    let wanted: HashSet<&str> = comps.iter().map(String::as_str).collect();

    // ── 1. drop *only* the component imports ────────────────────────────────
    let import_re = Regex::new(r#"(?m)^(?P<line>\s*import\s+[^;]+?;[ \t]*)$"#).unwrap();
    let mut stage1 = import_re
        .replace_all(tsx, |caps: &regex::Captures<'_>| {
            let line = &caps["line"];
            if wanted.iter().any(|c| line.contains(c)) {
                "".to_string()
            } else {
                line.to_string()
            }
        })
        .to_string();

    // ── 2. strip every export token/statement throughout the file ───────────
    stage1 = strip_exports(&stage1);

    // ── 3. work out where the principal component ends ──────────────────────
    let main_name = principal_component_name(tsx);
    let insert_at = if let Some(name) = main_name {
        end_of_component(&stage1, &name).unwrap_or(stage1.len())
    } else {
        stage1.len()
    };

    // ── 4. build the merged block with the requested inline components ──────
    let mut merged_block = String::from("\n//<merged\n");
    for (name, code) in comp_map {
        if wanted.contains(*name) {
            merged_block.push_str(&strip_exports(code));
            merged_block.push('\n');
        }
    }

    // ── 5. splice it in ─────────────────────────────────────────────────────
    let (head, tail) = stage1.split_at(insert_at);
    format!("{head}{merged_block}{tail}")
}

pub fn strip_exports(source: &str) -> String {
    // 1️⃣ kill "export { Foo, Bar };" & "export * from …"
    let tmp = Regex::new(r#"(?m)^\s*export\s+(\*|\{[^}]*\})[^{;\n]*;?\s*$"#)
        .unwrap()
        .replace_all(source, "");

    // 2️⃣ drop leading "export default "  (keep what follows)
    let tmp = Regex::new(r#"(?m)^\s*export\s+default\s+"#)
        .unwrap()
        .replace_all(&tmp, "");

    // 3️⃣ drop leading "export " (const|function|class|type|interface …)
    Regex::new(r#"(?m)^\s*export\s+"#)
        .unwrap()
        .replace_all(&tmp, "")
        .to_string()
}

fn principal_component_name(tsx: &str) -> Option<String> {
    // export default function|class NAME
    let re = Regex::new(r#"export\s+default\s+(?:function|class)\s+([A-Z][A-Za-z0-9_]*)"#).unwrap();
    if let Some(c) = re.captures(tsx) {
        return Some(c[1].to_string());
    }

    // export default NAME
    let re = Regex::new(r#"export\s+default\s+([A-Z][A-Za-z0-9_]*)"#).unwrap();
    if let Some(c) = re.captures(tsx) {
        return Some(c[1].to_string());
    }

    // export (const|function|class) NAME
    let re = Regex::new(r#"export\s+(?:const|function|class)\s+([A-Z][A-Za-z0-9_]*)"#).unwrap();
    re.captures(tsx).map(|c| c[1].to_string())
}

pub fn end_of_component(src: &str, name: &str) -> Option<usize> {
    // locate "function|class|const NAME"
    let re = Regex::new(&format!(
        r#"(function|class|const)\s+{}\b"#,
        regex::escape(name)
    ))
    .unwrap();

    let m = re.find(src)?;
    let mut i = src[m.end()..].find('{')? + m.end(); // index of first '{'

    // walk braces
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

pub fn merge_recurse(tsx: &str, repo: &str) -> String {
    let imported_components = get_imported_components(tsx);

    let tsx_without_export = strip_exports(tsx);

    if imported_components.len() == 0 {
        return tsx_without_export.to_string();
    }

    let mut hash_map: HashMap<String, String> = HashMap::new();

    for (i, component) in imported_components.iter().enumerate() {
        let path = format!("/etc/compo-doc/tmp/{repo}/components/{component}.tsx");

        let _ = match read_from_file_ut(&path) {
            Ok(res) => {
                let merged = merge_recurse(&res, repo);
                hash_map.insert(component.to_string(), merged)
            }
            Err(_) => {
                continue;
            }
        };
    }

    let inlined = inline_components(&tsx_without_export, &convert_hash(&hash_map));

    return inlined;
}
