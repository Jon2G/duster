//! Cross-platform path helpers and normalization

use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::path::Component;

/// Strip Windows extended-length (`\\?\`) / UNC prefixes for comparisons.
pub fn normalize_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();

    #[cfg(windows)]
    {
        let stripped = if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{}", rest)
        } else if let Some(rest) = s.strip_prefix(r"\\?\") {
            rest.to_string()
        } else {
            s.to_string()
        };
        PathBuf::from(stripped)
    }

    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

/// Case-insensitive path prefix check (case-insensitive on Windows).
pub fn path_starts_with(path: &Path, prefix: &Path) -> bool {
    let path = normalize_path(path);
    let prefix = normalize_path(prefix);

    #[cfg(windows)]
    {
        let path_str = path.to_string_lossy().to_lowercase();
        let prefix_str = prefix.to_string_lossy().to_lowercase();
        if path_str == prefix_str {
            return true;
        }
        let prefix_with_sep = if prefix_str.ends_with('\\') || prefix_str.ends_with('/') {
            prefix_str
        } else {
            format!("{}\\", prefix_str.trim_end_matches(['/', '\\']))
        };
        path_str.starts_with(&prefix_with_sep)
    }

    #[cfg(not(windows))]
    {
        path.starts_with(&prefix)
    }
}

/// Temp directories appropriate for the current platform.
pub fn temp_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let std_temp = std::env::temp_dir();
    if std_temp.exists() {
        dirs.push(normalize_path(&std_temp));
    }

    #[cfg(unix)]
    {
        for candidate in ["/tmp", "/var/tmp"] {
            let p = PathBuf::from(candidate);
            if p.exists() && !dirs.iter().any(|d| d == &p) {
                dirs.push(p);
            }
        }

        if let Ok(tmpdir) = std::env::var("TMPDIR") {
            let p = PathBuf::from(&tmpdir);
            if p.exists() && !dirs.iter().any(|d| path_starts_with(&p, d) || d == &p) {
                dirs.push(p);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let user_tmp = home.join("Library").join("Caches").join("TemporaryItems");
            if user_tmp.exists() && !dirs.iter().any(|d| d == &user_tmp) {
                dirs.push(user_tmp);
            }
        }
    }

    dirs
}

/// Default cache roots for scanning (safe / conventional locations only).
pub fn cache_roots() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let library_caches = home.join("Library").join("Caches");
            if library_caches.exists() {
                dirs.push(library_caches);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Do not treat whole LocalAppData as a single cache root — known caches +
        // --include-sensitive handle AppData. Keep ~/.cache for portable tools.
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = dirs::home_dir() {
            let cache_dir = home.join(".cache");
            if cache_dir.exists() {
                dirs.push(cache_dir);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(home) = dirs::home_dir() {
            let cache_dir = home.join(".cache");
            if cache_dir.exists() {
                dirs.push(cache_dir);
            }
        }
    }

    dirs
}

/// Local and Roaming AppData roots (Windows). Empty on other platforms.
pub fn appdata_roots() -> Vec<(PathBuf, &'static str)> {
    let mut roots = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Some(local) = dirs::cache_dir() {
            if local.exists() {
                roots.push((local, "Local AppData"));
            }
        }
        if let Some(roaming) = dirs::config_dir() {
            // config_dir is typically %APPDATA% (Roaming)
            if roaming.exists() {
                roots.push((roaming, "Roaming AppData"));
            }
        }
    }

    roots
}

/// Paths that must never be offered as sensitive AppData clean targets.
pub fn is_appdata_denylisted(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    const DENY: &[&str] = &[
        "microsoft\\credentials",
        "microsoft\\protect",
        "microsoft\\crypto",
        "microsoft\\systemcertificates",
        "\\login data",
        "\\logins.json",
        "\\cookies",
        "\\key3.db",
        "\\key4.db",
        "\\duster\\",
    ];
    DENY.iter().any(|d| lower.contains(d))
}

/// Trash / Recycle Bin directories for the current platform.
pub fn trash_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Some(home) = dirs::home_dir() {
        #[cfg(target_os = "macos")]
        {
            let trash = home.join(".Trash");
            if trash.exists() {
                dirs.push(trash);
            }
        }

        #[cfg(target_os = "linux")]
        {
            let trash = home.join(".local/share/Trash/files");
            if trash.exists() {
                dirs.push(trash);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Per-volume recycle bins; may require elevation to read.
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\$Recycle.Bin", letter as char);
            let p = PathBuf::from(&root);
            if p.exists() {
                dirs.push(p);
            }
        }
    }

    dirs
}

/// True if `path` is under a known temp directory.
pub fn is_under_temp(path: &Path) -> bool {
    temp_dirs().iter().any(|t| path_starts_with(path, t))
}

/// True if `path` looks like a recycle-bin entry.
pub fn is_under_trash(path: &Path) -> bool {
    trash_dirs().iter().any(|t| path_starts_with(path, t))
}

/// Windows system / protected path that must never be deleted.
pub fn is_protected_system_path(path: &Path) -> bool {
    let normalized = normalize_path(path);
    let lower = normalized.to_string_lossy().to_lowercase();

    #[cfg(windows)]
    {
        if lower.contains("\\windows\\") || lower.ends_with("\\windows") {
            return true;
        }
        if lower.contains("\\program files\\") || lower.contains("\\program files (x86)\\") {
            return true;
        }
        // Drive root itself
        let comps: Vec<_> = normalized.components().collect();
        if comps.len() <= 1 {
            return true;
        }
        if comps.len() == 2 {
            if let Component::Normal(name) = comps[1] {
                let n = name.to_string_lossy().to_lowercase();
                if n == "$recycle.bin" {
                    return false; // allow recycle bin contents via trash allowlist
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (normalized, lower);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_verbatim_prefix() {
        #[cfg(windows)]
        {
            let p = PathBuf::from(r"\\?\C:\Users\test");
            assert_eq!(normalize_path(&p), PathBuf::from(r"C:\Users\test"));
        }
        #[cfg(not(windows))]
        {
            let p = PathBuf::from("/tmp/foo");
            assert_eq!(normalize_path(&p), p);
        }
    }

    #[test]
    fn path_starts_with_basic() {
        let home = PathBuf::from(if cfg!(windows) {
            r"C:\Users\test"
        } else {
            "/home/test"
        });
        let child = home.join("AppData").join("Local");
        assert!(path_starts_with(&child, &home));
    }

    #[test]
    fn temp_dirs_nonempty() {
        assert!(!temp_dirs().is_empty() || !std::env::temp_dir().exists());
    }
}
