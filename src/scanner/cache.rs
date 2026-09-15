//! System and application cache scanner

use super::{calculate_dir_size, get_last_accessed, Category, CleanableFile, RiskLevel, Scanner};
use crate::config::Config;
use crate::platform;
use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;

pub struct CacheScanner;

impl CacheScanner {
    pub fn new() -> Self {
        Self
    }

    /// Get cache directories to scan based on the platform
    fn get_cache_dirs(&self, config: &Config) -> Vec<PathBuf> {
        let mut dirs = platform::cache_roots();

        // Add any custom cache paths from config
        for path in &config.cache_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                dirs.push(p);
            }
        }

        dirs
    }
}

impl Default for CacheScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner for CacheScanner {
    fn name(&self) -> &'static str {
        "Cache Scanner"
    }

    fn scan(&self, config: &Config) -> Result<Vec<CleanableFile>> {
        let mut results = Vec::new();
        let cache_dirs = self.get_cache_dirs(config);

        for cache_dir in cache_dirs {
            // Scan top-level directories in cache
            let entries = match std::fs::read_dir(&cache_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();

                // Skip if excluded
                if config.is_excluded(&path) {
                    continue;
                }

                // Calculate size
                let size = if path.is_dir() {
                    calculate_dir_size(&path)
                } else {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                };

                // Skip very small cache entries (less than 1MB)
                if size < 1024 * 1024 {
                    continue;
                }

                let last_accessed = get_last_accessed(&path).unwrap_or_else(Utc::now);

                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                results.push(CleanableFile {
                    path: path.clone(),
                    size,
                    category: Category::Cache,
                    last_accessed,
                    reason: format!("Cache directory: {}", name),
                    is_directory: path.is_dir(),
                    risk: RiskLevel::Normal,
                });
            }
        }

        // Sort by size descending
        results.sort_by(|a, b| b.size.cmp(&a.size));

        Ok(results)
    }
}

/// Scan for specific application caches that are known to be safe to delete
pub struct KnownCacheScanner;

impl KnownCacheScanner {
    pub fn new() -> Self {
        Self
    }

    /// Cross-platform package-manager / toolchain caches relative to home
    fn home_relative_caches() -> Vec<(&'static str, &'static str)> {
        vec![
            (".npm/_cacache", "npm cache"),
            (".yarn/cache", "Yarn cache"),
            (".pnpm-store", "pnpm cache"),
            (".cargo/registry/cache", "Cargo registry cache"),
            (".gradle/caches", "Gradle cache"),
            (".m2/repository", "Maven cache"),
            (".nuget/packages", "NuGet cache"),
            (".cache/pip", "pip cache"),
            (".cache/go-build", "Go build cache"),
            (".vscode-server", "VS Code Server"),
        ]
    }

    #[cfg(target_os = "macos")]
    fn macos_caches() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Library/Caches/Homebrew", "Homebrew downloads cache"),
            ("Library/Caches/com.apple.dt.Xcode", "Xcode cache"),
            ("Library/Caches/JetBrains", "JetBrains IDEs cache"),
            ("Library/Caches/com.microsoft.VSCode", "VS Code cache"),
            ("Library/Caches/com.google.Chrome", "Chrome browser cache"),
            ("Library/Caches/com.brave.Browser", "Brave browser cache"),
            ("Library/Caches/org.mozilla.firefox", "Firefox browser cache"),
            ("Library/Caches/com.apple.Safari", "Safari browser cache"),
            ("Library/Caches/com.spotify.client", "Spotify cache"),
            ("Library/Caches/com.docker.docker", "Docker cache"),
            ("Library/Caches/Slack", "Slack cache"),
        ]
    }

    /// Absolute / env-based Windows known caches
    #[cfg(target_os = "windows")]
    fn windows_absolute_caches() -> Vec<(PathBuf, &'static str)> {
        let mut list = Vec::new();

        if let Some(local) = dirs::cache_dir() {
            let candidates = [
                (local.join("npm-cache"), "npm cache"),
                (local.join("pip").join("Cache"), "pip cache"),
                (local.join("go-build"), "Go build cache"),
                (
                    local
                        .join("Google")
                        .join("Chrome")
                        .join("User Data")
                        .join("Default")
                        .join("Cache"),
                    "Chrome browser cache",
                ),
                (
                    local
                        .join("Microsoft")
                        .join("Edge")
                        .join("User Data")
                        .join("Default")
                        .join("Cache"),
                    "Edge browser cache",
                ),
                (local.join("JetBrains"), "JetBrains IDEs cache"),
                (local.join("Docker"), "Docker Desktop data"),
            ];
            list.extend(candidates);
        }

        if let Some(roaming) = dirs::config_dir() {
            let candidates = [
                (roaming.join("npm-cache"), "npm cache (Roaming)"),
                (roaming.join("Code").join("Cache"), "VS Code cache"),
                (roaming.join("Code").join("CachedData"), "VS Code cached data"),
                (
                    roaming.join("Code").join("CachedExtensions"),
                    "VS Code cached extensions",
                ),
                (roaming.join("Cursor").join("Cache"), "Cursor cache"),
                (roaming.join("Cursor").join("CachedData"), "Cursor cached data"),
            ];
            list.extend(candidates);
        }

        list
    }
}

impl Default for KnownCacheScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner for KnownCacheScanner {
    fn name(&self) -> &'static str {
        "Known Cache Scanner"
    }

    fn scan(&self, config: &Config) -> Result<Vec<CleanableFile>> {
        let mut results = Vec::new();

        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return Ok(results),
        };

        let mut entries: Vec<(PathBuf, String)> = Self::home_relative_caches()
            .into_iter()
            .map(|(rel, desc)| (home.join(rel), desc.to_string()))
            .collect();

        #[cfg(target_os = "macos")]
        {
            entries.extend(
                Self::macos_caches()
                    .into_iter()
                    .map(|(rel, desc)| (home.join(rel), desc.to_string())),
            );
        }

        #[cfg(target_os = "windows")]
        {
            entries.extend(
                Self::windows_absolute_caches()
                    .into_iter()
                    .map(|(path, desc)| (path, desc.to_string())),
            );
        }

        for (path, description) in entries {
            if !path.exists() {
                continue;
            }

            if config.is_excluded(&path) {
                continue;
            }

            let size = calculate_dir_size(&path);
            let last_accessed = get_last_accessed(&path).unwrap_or_else(Utc::now);

            // Only include if it's at least 10MB
            if size >= 10 * 1024 * 1024 {
                results.push(CleanableFile {
                    path,
                    size,
                    category: Category::Cache,
                    last_accessed,
                    reason: description,
                    is_directory: true,
                    risk: RiskLevel::Normal,
                });
            }
        }

        results.sort_by(|a, b| b.size.cmp(&a.size));

        Ok(results)
    }
}
