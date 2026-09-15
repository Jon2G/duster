//! Opt-in sensitive AppData scanner (Windows Local/Roaming)

use super::{calculate_dir_size, get_last_accessed, Category, CleanableFile, RiskLevel, Scanner};
use crate::config::Config;
use crate::platform;
use anyhow::Result;
use chrono::Utc;

pub struct AppDataScanner;

impl AppDataScanner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AppDataScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner for AppDataScanner {
    fn name(&self) -> &'static str {
        "AppData Scanner"
    }

    fn scan(&self, config: &Config) -> Result<Vec<CleanableFile>> {
        let mut results = Vec::new();

        if !config.include_sensitive {
            return Ok(results);
        }

        for (root, root_label) in platform::appdata_roots() {
            let entries = match std::fs::read_dir(&root) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();

                if config.is_excluded(&path) {
                    continue;
                }

                if platform::is_appdata_denylisted(&path) {
                    continue;
                }

                // Skip Temp — handled by TempScanner
                if let Some(name) = path.file_name() {
                    if name.eq_ignore_ascii_case("Temp") || name.eq_ignore_ascii_case("tmp") {
                        continue;
                    }
                }

                let is_dir = path.is_dir();
                let size = if is_dir {
                    calculate_dir_size(&path)
                } else {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                };

                // Skip tiny entries
                if size < 5 * 1024 * 1024 {
                    continue;
                }

                let last_accessed = get_last_accessed(&path).unwrap_or_else(Utc::now);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                results.push(CleanableFile {
                    path,
                    size,
                    category: Category::Cache,
                    last_accessed,
                    reason: format!(
                        "[sensitive] {} entry: {} — may contain app settings or credentials",
                        root_label, name
                    ),
                    is_directory: is_dir,
                    risk: RiskLevel::Sensitive,
                });
            }
        }

        results.sort_by(|a, b| b.size.cmp(&a.size));
        Ok(results)
    }
}
