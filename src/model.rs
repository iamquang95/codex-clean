use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Go,
    Node,
    Python,
    Unknown,
}

impl ProjectType {
    pub fn artifact_dirs(&self) -> &[&str] {
        match self {
            Self::Rust => &["target"],
            Self::Go => &["vendor"],
            Self::Node => &["node_modules"],
            Self::Python => &[".venv", "__pycache__"],
            Self::Unknown => &["target", "node_modules", ".venv", "build", "dist"],
        }
    }
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::Go => write!(f, "Go"),
            Self::Node => write!(f, "Node"),
            Self::Python => write!(f, "Python"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

pub fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < units.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{size:.0}{}", units[unit_idx])
    } else {
        format!("{size:.1}{}", units[unit_idx])
    }
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub codex_id: String,
    pub path: PathBuf,
    pub project_name: String,
    pub project_path: PathBuf,
    pub git_worktree_path: Option<PathBuf>,
    pub branch: Option<String>,
    pub thread_id: Option<String>,
    pub thread_name: Option<String>,
    pub updated_at: Option<String>,
    pub total_size: u64,
    pub artifact_size: u64,
    pub project_type: ProjectType,
    pub selected: bool,
}

impl WorktreeInfo {
    pub fn display_branch(&self) -> &str {
        self.branch.as_deref().unwrap_or("(unknown)")
    }

    pub fn display_thread(&self) -> &str {
        self.thread_name
            .as_deref()
            .or(self.thread_id.as_deref())
            .unwrap_or("(unknown)")
    }

    pub fn display_updated_at(&self) -> String {
        match &self.updated_at {
            Some(ts) => relative_time(ts),
            None => "(unknown)".to_string(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.project_name == "(empty)"
    }
}

fn relative_time(iso_str: &str) -> String {
    let now = std::time::SystemTime::now();
    let epoch_now = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(epoch_ts) = parse_iso_timestamp(iso_str) {
        let diff = epoch_now.saturating_sub(epoch_ts);
        if diff < 60 {
            "just now".to_string()
        } else if diff < 3600 {
            let mins = diff / 60;
            format!("{mins}m ago")
        } else if diff < 86400 {
            let hours = diff / 3600;
            format!("{hours}h ago")
        } else if diff < 604800 {
            let days = diff / 86400;
            format!("{days}d ago")
        } else if diff < 2592000 {
            let weeks = diff / 604800;
            format!("{weeks}w ago")
        } else {
            iso_str[..10].to_string()
        }
    } else {
        iso_str.to_string()
    }
}

fn parse_iso_timestamp(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let year: u64 = s[0..4].parse().ok()?;
    let month: u64 = s[5..7].parse().ok()?;
    let day: u64 = s[8..10].parse().ok()?;
    let hour: u64 = s[11..13].parse().ok()?;
    let min: u64 = s[14..16].parse().ok()?;
    let sec: u64 = s[17..19].parse().ok()?;

    // Reject non-UTC timestamps (offsets like +05:30 would be parsed incorrectly)
    if s.len() > 19 && !s[19..].starts_with('Z') && !s[19..].starts_with('.') {
        return None;
    }

    if month < 1 || month > 12 || day < 1 || day > 31 || hour >= 24 || min >= 60 || sec >= 60 {
        return None;
    }

    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += month_days[(m - 1) as usize] as u64;
        if m == 2 && is_leap_year(year) {
            days += 1;
        }
    }
    days += day - 1;

    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

fn is_leap_year(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[derive(Debug, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    #[serde(default)]
    pub thread_name: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodexThread {
    #[serde(rename = "ownerThreadId")]
    pub owner_thread_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1_048_576), "1.0MB");
        assert_eq!(format_size(1_073_741_824), "1.0GB");
        assert_eq!(format_size(2_365_587_456), "2.2GB");
    }

    #[test]
    fn test_relative_time() {
        let result = relative_time("2025-03-20T10:30:00Z");
        assert!(!result.is_empty());

        let result = relative_time("invalid");
        assert_eq!(result, "invalid");
    }

    #[test]
    fn test_parse_iso_timestamp() {
        assert!(parse_iso_timestamp("2025-03-20T10:30:00Z").is_some());
        assert!(parse_iso_timestamp("2025-03-20T10:30:00.000Z").is_some());
        assert!(parse_iso_timestamp("bad").is_none());
    }

    #[test]
    fn test_parse_iso_rejects_invalid() {
        assert!(parse_iso_timestamp("2025-13-20T10:30:00Z").is_none()); // month 13
        assert!(parse_iso_timestamp("2025-03-20T25:30:00Z").is_none()); // hour 25
        assert!(parse_iso_timestamp("2025-03-20T10:30:00+05:30").is_none()); // non-UTC offset
    }

    #[test]
    fn test_artifact_dirs() {
        assert_eq!(ProjectType::Rust.artifact_dirs(), &["target"]);
        assert_eq!(ProjectType::Go.artifact_dirs(), &["vendor"]);
        assert_eq!(ProjectType::Python.artifact_dirs(), &[".venv", "__pycache__"]);
    }
}
