//! Best-effort path hints for prototype / logical identities.
//! Real activity capture should pass absolute executable paths or AUMIDs.

use std::path::{Path, PathBuf};

pub fn resolve_known_executable(logical_or_path: &str) -> Option<PathBuf> {
    let trimmed = logical_or_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let as_path = PathBuf::from(trimmed);
    if as_path.is_file() {
        return Some(as_path);
    }

    let key = trimmed
        .trim_start_matches("app:")
        .to_ascii_lowercase()
        .replace('_', "-");

    let candidates = candidate_paths_for(&key);
    candidates.into_iter().find(|p| p.is_file())
}

fn candidate_paths_for(key: &str) -> Vec<PathBuf> {
    let local = env_path("LOCALAPPDATA");
    let roaming = env_path("APPDATA");
    let pf = env_path("ProgramFiles");
    let pf86 = env_path("ProgramFiles(x86)");
    let user = env_path("USERPROFILE");

    let mut paths = Vec::new();

    match key {
        "vscode" | "code" | "vs-code" => {
            push(
                &mut paths,
                [
                    join(&local, r"Programs\Microsoft VS Code\Code.exe"),
                    join(&pf, r"Microsoft VS Code\Code.exe"),
                    join(&pf86, r"Microsoft VS Code\Code.exe"),
                ],
            );
        }
        "chrome" | "google-chrome" => {
            push(
                &mut paths,
                [
                    join(&pf, r"Google\Chrome\Application\chrome.exe"),
                    join(&pf86, r"Google\Chrome\Application\chrome.exe"),
                    join(&local, r"Google\Chrome\Application\chrome.exe"),
                ],
            );
        }
        "msedge" | "edge" | "microsoft-edge" => {
            push(
                &mut paths,
                [
                    join(&pf, r"Microsoft\Edge\Application\msedge.exe"),
                    join(&pf86, r"Microsoft\Edge\Application\msedge.exe"),
                ],
            );
        }
        "explorer" | "file-explorer" => {
            if let Ok(windir) = std::env::var("WINDIR") {
                paths.push(PathBuf::from(windir).join("explorer.exe"));
            }
            paths.push(PathBuf::from(r"C:\Windows\explorer.exe"));
        }
        "chatgpt" => {
            // Store / MSIX style installs under WindowsApps are ACL-protected;
            // also try user-local packages and shortcuts.
            push(
                &mut paths,
                [
                    join(&local, r"Microsoft\WindowsApps\ChatGPT.exe"),
                    join(&local, r"Programs\ChatGPT\ChatGPT.exe"),
                ],
            );
        }
        "claude" | "claude-code" => {
            push(
                &mut paths,
                [
                    join(&local, r"AnthropicClaude\claude.exe"),
                    join(&local, r"Programs\claude\Claude.exe"),
                    join(&roaming, r"Claude\Claude.exe"),
                ],
            );
        }
        "codex" => {
            // Codex may be packaged (MSIX) or run via other hosts — path is best-effort.
            push(
                &mut paths,
                [
                    join(&local, r"Programs\Codex\Codex.exe"),
                    join(&local, r"Microsoft\WindowsApps\Codex.exe"),
                ],
            );
        }
        "typeless" => {
            push(
                &mut paths,
                [
                    join(&local, r"Programs\Typeless\Typeless.exe"),
                    join(&pf, r"Typeless\Typeless.exe"),
                    join(&user, r"AppData\Local\Programs\Typeless\Typeless.exe"),
                ],
            );
        }
        "wechat" | "weixin" => {
            push(
                &mut paths,
                [
                    join(&pf, r"Tencent\WeChat\WeChat.exe"),
                    join(&pf86, r"Tencent\WeChat\WeChat.exe"),
                    join(&local, r"Tencent\WeChat\WeChat.exe"),
                ],
            );
        }
        "youtube" => {
            // YouTube is typically a site/PWA — fall through; chrome path used as host elsewhere.
        }
        _ => {}
    }

    paths
}

fn env_path(key: &str) -> PathBuf {
    std::env::var_os(key).map(PathBuf::from).unwrap_or_default()
}

fn join(base: &Path, relative: &str) -> PathBuf {
    if base.as_os_str().is_empty() {
        PathBuf::from(relative)
    } else {
        base.join(relative)
    }
}

fn push(out: &mut Vec<PathBuf>, items: impl IntoIterator<Item = PathBuf>) {
    out.extend(items);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_for_unknown_logical_key() {
        assert!(resolve_known_executable("app:not-a-real-app-xyz").is_none());
    }
}
