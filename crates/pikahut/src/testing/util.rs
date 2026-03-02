use std::path::{Path, PathBuf};

pub(crate) fn command_exists(binary: &str) -> bool {
    let candidate = Path::new(binary);
    if candidate.is_absolute() || binary.contains('/') {
        return candidate.is_file();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths)
        .map(|dir| dir.join(binary))
        .any(|path| path.is_file())
}

pub(crate) fn env_truthy(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

pub(crate) fn env_var_present(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

pub(crate) fn non_empty_env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn default_code_checkout_dir(repo_name: &str) -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        return home.join("code").join(repo_name);
    }
    PathBuf::from("code").join(repo_name)
}

pub(crate) fn resolve_openclaw_dir_default(workspace_root: &Path) -> PathBuf {
    let direct = workspace_root.join("openclaw");
    if direct.join("package.json").is_file() {
        return direct;
    }

    if let Some(parent) = workspace_root.parent() {
        let sibling = parent.join("openclaw");
        if sibling.join("package.json").is_file() {
            return sibling;
        }
    }

    direct
}
