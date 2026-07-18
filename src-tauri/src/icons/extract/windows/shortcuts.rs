use std::path::{Path, PathBuf};

pub(super) fn find_start_menu_shortcut(executable: &Path) -> Option<PathBuf> {
    let executable_name = executable.file_stem()?.to_string_lossy();
    find_shortcut(&executable_name)
}

pub(super) fn find_shortcut_by_identity(identity: &str) -> Option<PathBuf> {
    let logical = identity.strip_prefix("app:").unwrap_or(identity);
    find_shortcut(logical)
}

fn find_shortcut(query: &str) -> Option<PathBuf> {
    let query_keys = name_keys(query);
    if query_keys.is_empty() {
        return None;
    }
    let mut best: Option<(u8, PathBuf)> = None;
    for root in shortcut_roots() {
        walk_shortcuts(&root, &query_keys, 5, &mut best);
    }
    best.map(|(_, path)| path)
}

fn shortcut_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let appdata = PathBuf::from(appdata);
        roots.push(appdata.join(r"Microsoft\Windows\Start Menu\Programs"));
        roots.push(appdata.join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar"));
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(user) = std::env::var_os("USERPROFILE") {
        roots.push(PathBuf::from(user).join("Desktop"));
    }
    if let Some(public) = std::env::var_os("PUBLIC") {
        roots.push(PathBuf::from(public).join("Desktop"));
    }
    roots
}

fn walk_shortcuts(
    directory: &Path,
    query_keys: &[String],
    depth: u8,
    best: &mut Option<(u8, PathBuf)>,
) {
    if depth == 0 || !directory.is_dir() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_shortcuts(&path, query_keys, depth - 1, best);
            continue;
        }
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
        {
            continue;
        }
        let name = path
            .file_stem()
            .map(|value| value.to_string_lossy())
            .unwrap_or_default();
        let score = match_score(query_keys, &name_keys(&name));
        if score > best.as_ref().map_or(0, |current| current.0) {
            *best = Some((score, path));
        }
    }
}

fn name_keys(value: &str) -> Vec<String> {
    let words = value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return Vec::new();
    }
    let compact = words.join("");
    let mut keys = vec![compact.clone()];
    for prefix in ["microsoft", "google", "adobe"] {
        if let Some(stripped) = compact
            .strip_prefix(prefix)
            .filter(|value| value.len() >= 3)
        {
            keys.push(stripped.to_string());
        }
    }
    keys.extend(words.into_iter().filter(|word| word.chars().count() >= 4));
    keys.sort();
    keys.dedup();
    keys
}

fn match_score(query: &[String], candidate: &[String]) -> u8 {
    query
        .iter()
        .flat_map(|left| candidate.iter().map(move |right| (left, right)))
        .map(|(left, right)| {
            if left == right {
                100
            } else if left.chars().count().min(right.chars().count()) >= 4
                && (left.contains(right) || right.contains(left))
            {
                72
            } else {
                0
            }
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_registered_names_with_vendor_prefixes() {
        assert_eq!(
            match_score(&name_keys("Microsoft Word"), &name_keys("Word")),
            100
        );
        assert_eq!(
            match_score(&name_keys("app:notion"), &name_keys("Notion")),
            100
        );
    }

    #[test]
    fn rejects_unrelated_shortcut_names() {
        assert_eq!(
            match_score(&name_keys("Notion"), &name_keys("Calculator")),
            0
        );
    }
}
