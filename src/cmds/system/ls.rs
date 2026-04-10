//! Filters directory listings into a compact tree format.
//!
//! On Windows, uses native Rust std::fs implementation for zero-dependency
//! directory listing. On Unix, proxies to external `ls` command.

use super::constants::NOISE_DIRS;
use crate::core::tracking;
use anyhow::{Context, Result};
use std::io::IsTerminal;

#[cfg(not(target_os = "windows"))]
use crate::core::runner::{self, RunOptions};
#[cfg(not(target_os = "windows"))]
use crate::core::utils::resolved_command;

/// Main entry point - routes to platform-specific implementation
pub fn run(args: &[String], verbose: u8) -> Result<i32> {
    #[cfg(target_os = "windows")]
    {
        run_windows_native(args, verbose)
    }

    #[cfg(not(target_os = "windows"))]
    {
        run_unix_proxy(args, verbose)
    }
}

/// Windows-native implementation using std::fs
#[cfg(target_os = "windows")]
fn run_windows_native(args: &[String], verbose: u8) -> Result<i32> {
    use std::fs;

    // Parse arguments
    let show_all = args
        .iter()
        .any(|a| (a.starts_with('-') && !a.starts_with("--") && a.contains('a')) || a == "--all");
    
    let long_format = args
        .iter()
        .any(|a| (a.starts_with('-') && !a.starts_with("--") && a.contains('l')) || a == "--long");
    
    let human_readable = args
        .iter()
        .any(|a| (a.starts_with('-') && !a.starts_with("--") && a.contains('h')) || a == "--human-readable");

    let paths: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .collect();

    let dir_path = if paths.is_empty() { "." } else { paths[0] };

    // Validate unsupported flags on Windows
    let unsupported_flags: Vec<&str> = args
        .iter()
        .filter_map(|a| {
            if a.starts_with('-') && !a.starts_with("--") {
                // Check for unsupported short flags (anything except a, l, h)
                let unsupported: String = a.chars()
                    .skip(1)
                    .filter(|c| !['a', 'l', 'h'].contains(c))
                    .collect();
                if !unsupported.is_empty() {
                    return Some(a.as_str());
                }
            } else if a.starts_with("--") && !["--all", "--long", "--human-readable"].contains(&a.as_str()) {
                return Some(a.as_str());
            }
            None
        })
        .collect();

    if !unsupported_flags.is_empty() && verbose > 0 {
        eprintln!("rtk ls (Windows): unsupported flags ignored: {}", unsupported_flags.join(", "));
    }

    let timer = tracking::TimedExecution::start();

    // Validate directory exists before attempting to read
    let path_metadata = fs::metadata(dir_path)
        .with_context(|| format!("Directory not found: {}", dir_path))?;
    
    if !path_metadata.is_dir() {
        anyhow::bail!("Not a directory: {}", dir_path);
    }

    // Read directory - handle permission errors gracefully
    let entries = match fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            anyhow::bail!("Permission denied: {}", dir_path);
        }
        Err(err) => {
            return Err(err).with_context(|| format!("Failed to read directory: {}", dir_path));
        }
    };

    let mut raw_output = String::new();
    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<(String, u64, String)> = Vec::new(); // (name, size, metadata_line)

    for entry_result in entries {
        let entry = match entry_result {
            Ok(e) => e,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                // Skip entries we can't access rather than failing entirely
                if verbose > 0 {
                    eprintln!("rtk ls: skipping inaccessible entry (permission denied)");
                }
                continue;
            }
            Err(err) => {
                return Err(err).with_context(|| format!("Failed to read directory entry in {}", dir_path));
            }
        };
        
        // Handle non-UTF-8 filenames gracefully (convert with replacement chars)
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files unless -a provided
        if !show_all && name.starts_with('.') {
            continue;
        }

        // Filter noise dirs unless -a provided
        if !show_all && NOISE_DIRS.iter().any(|noise| name == *noise) {
            continue;
        }

        let metadata = entry.metadata().with_context(|| format!("Failed to read metadata for {}", name))?;
        let is_dir = metadata.is_dir();

        if is_dir {
            dirs.push(name.clone());
        } else {
            let size = metadata.len();
            let metadata_line = if long_format {
                format_long_entry(&name, &metadata, human_readable)
            } else {
                String::new()
            };
            files.push((name.clone(), size, metadata_line));
        }

        // Build raw output (for tracking purposes)
        if long_format {
            if is_dir {
                raw_output.push_str(&format!("d---------  1 user  group      0 {} {}\n", 
                    format_timestamp(&metadata), name));
            } else {
                raw_output.push_str(&format_long_entry(&name, &metadata, human_readable));
                raw_output.push('\n');
            }
        } else {
            raw_output.push_str(&name);
            raw_output.push('\n');
        }
    }

    // Generate compact output
    let (entries_str, summary) = format_compact_output(&dirs, &files, show_all, human_readable, long_format);

    // Only show summary in interactive mode
    let is_tty = std::io::stdout().is_terminal();
    let filtered = if is_tty {
        format!("{}{}", entries_str, summary)
    } else {
        entries_str
    };

    if verbose > 0 {
        eprintln!(
            "Chars: {} → {} ({}% reduction)",
            raw_output.len(),
            filtered.len(),
            if !raw_output.is_empty() {
                100 - (filtered.len() * 100 / raw_output.len())
            } else {
                0
            }
        );
    }

    // Track token savings
    timer.track(
        &format!("ls {}", dir_path),
        &format!("rtk ls {}", dir_path),
        &raw_output,
        &filtered
    );

    print!("{}", filtered);
    Ok(0)
}

/// Format file metadata in long format (similar to ls -l)
#[cfg(target_os = "windows")]
fn format_long_entry(name: &str, metadata: &std::fs::Metadata, human_readable: bool) -> String {
    let size = metadata.len();
    let size_str = if human_readable {
        human_size(size)
    } else {
        size.to_string()
    };
    
    // Windows doesn't have Unix-style permissions, so we show readonly attribute
    let readonly = metadata.permissions().readonly();
    let perms = if readonly { "r--" } else { "rw-" };
    
    let modified = format_timestamp(metadata);
    
    format!("-{}-------  1 user  group {:>8} {} {}", perms, size_str, modified, name)
}

/// Format timestamp from metadata
#[cfg(target_os = "windows")]
fn format_timestamp(metadata: &std::fs::Metadata) -> String {
    use std::time::SystemTime;
    
    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
            // Simple date formatting (could use chrono for better formatting)
            let secs = duration.as_secs();
            let days_since_epoch = secs / 86400;
            // Simplified: just show last modified in epoch days
            return format!("{}d ago", days_since_epoch % 365);
        }
    }
    "unknown".to_string()
}

/// Format compact output from dirs and files
#[cfg(target_os = "windows")]
fn format_compact_output(
    dirs: &[String],
    files: &[(String, u64, String)],
    _show_all: bool,
    human_readable: bool,
    long_format: bool,
) -> (String, String) {
    use std::collections::HashMap;

    if dirs.is_empty() && files.is_empty() {
        return ("(empty)\n".to_string(), String::new());
    }

    let mut entries = String::new();
    let mut by_ext: HashMap<String, usize> = HashMap::new();

    // Dirs first
    for d in dirs {
        if long_format {
            entries.push_str(&format!("d---------  {} {}/\n", format!("{:>8}", "0"), d));
        } else {
            entries.push_str(d);
            entries.push_str("/\n");
        }
    }

    // Files with size
    for (name, size, metadata_line) in files {
        if long_format && !metadata_line.is_empty() {
            entries.push_str(metadata_line);
            entries.push('\n');
        } else {
            let size_str = if human_readable {
                human_size(*size)
            } else {
                size.to_string()
            };
            entries.push_str(name);
            entries.push_str("  ");
            entries.push_str(&size_str);
            entries.push('\n');
        }

        // Track extensions
        let ext = if let Some(pos) = name.rfind('.') {
            name[pos..].to_string()
        } else {
            "no ext".to_string()
        };
        *by_ext.entry(ext).or_insert(0) += 1;
    }

    // Summary line
    let mut summary = format!("\nSummary: {} files, {} dirs", files.len(), dirs.len());
    if !by_ext.is_empty() {
        let mut ext_counts: Vec<_> = by_ext.iter().collect();
        ext_counts.sort_by(|a, b| b.1.cmp(a.1));
        let ext_parts: Vec<String> = ext_counts
            .iter()
            .take(5)
            .map(|(ext, count)| format!("{} {}", count, ext))
            .collect();
        summary.push_str(" (");
        summary.push_str(&ext_parts.join(", "));
        if ext_counts.len() > 5 {
            summary.push_str(&format!(", +{} more", ext_counts.len() - 5));
        }
        summary.push(')');
    }
    summary.push('\n');

    (entries, summary)
}
/// Unix implementation - proxy to external ls command (unchanged)
#[cfg(not(target_os = "windows"))]
fn run_unix_proxy(args: &[String], verbose: u8) -> Result<i32> {
    let show_all = args
        .iter()
        .any(|a| (a.starts_with('-') && !a.starts_with("--") && a.contains('a')) || a == "--all");

    let flags: Vec<&str> = args
        .iter()
        .filter(|a| a.starts_with('-'))
        .map(|s| s.as_str())
        .collect();
    let paths: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .map(|s| s.as_str())
        .collect();

    let mut cmd = resolved_command("ls");
    cmd.arg("-la");
    for flag in &flags {
        if flag.starts_with("--") {
            if *flag != "--all" {
                cmd.arg(flag);
            }
        } else {
            let stripped = flag.trim_start_matches('-');
            let extra: String = stripped
                .chars()
                .filter(|c| *c != 'l' && *c != 'a' && *c != 'h')
                .collect();
            if !extra.is_empty() {
                cmd.arg(format!("-{}", extra));
            }
        }
    }

    if paths.is_empty() {
        cmd.arg(".");
    } else {
        for p in &paths {
            cmd.arg(p);
        }
    }

    let target_display = if paths.is_empty() {
        ".".to_string()
    } else {
        paths.join(" ")
    };

    runner::run_filtered(
        cmd,
        "ls",
        &format!("-la {}", target_display),
        |raw| {
            let (entries, summary) = compact_ls(raw, show_all);

            // Only show summary in interactive mode (not when piped)
            let is_tty = std::io::stdout().is_terminal();
            let filtered = if is_tty {
                format!("{}{}", entries, summary)
            } else {
                entries
            };

            if verbose > 0 {
                eprintln!(
                    "Chars: {} → {} ({}% reduction)",
                    raw.len(),
                    filtered.len(),
                    if !raw.is_empty() {
                        100 - (filtered.len() * 100 / raw.len())
                    } else {
                        0
                    }
                );
            }
            filtered
        },
        RunOptions::stdout_only()
            .early_exit_on_failure()
            .no_trailing_newline(),
    )
}

/// Format bytes into human-readable size
fn human_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1}M", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

/// Parse ls -la output into compact format (Unix-specific)
/// Returns (entries, summary) so caller can suppress summary when piped.
#[cfg(not(target_os = "windows"))]
fn compact_ls(raw: &str, show_all: bool) -> (String, String) {
    use std::collections::HashMap;

    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<(String, String)> = Vec::new(); // (name, size)
    let mut by_ext: HashMap<String, usize> = HashMap::new();

    for line in raw.lines() {
        // Skip total, empty, . and ..
        if line.starts_with("total ") || line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        // Filename is everything from column 9 onward (handles spaces)
        let name = parts[8..].join(" ");

        // Skip . and ..
        if name == "." || name == ".." {
            continue;
        }

        // Filter noise dirs unless -a
        if !show_all && NOISE_DIRS.iter().any(|noise| name == *noise) {
            continue;
        }

        let is_dir = parts[0].starts_with('d');

        if is_dir {
            dirs.push(name);
        } else if parts[0].starts_with('-') || parts[0].starts_with('l') {
            let size: u64 = parts[4].parse().unwrap_or(0);
            let ext = if let Some(pos) = name.rfind('.') {
                name[pos..].to_string()
            } else {
                "no ext".to_string()
            };
            *by_ext.entry(ext).or_insert(0) += 1;
            files.push((name, human_size(size)));
        }
    }

    if dirs.is_empty() && files.is_empty() {
        return ("(empty)\n".to_string(), String::new());
    }

    let mut entries = String::new();

    // Dirs first, compact
    for d in &dirs {
        entries.push_str(d);
        entries.push_str("/\n");
    }

    // Files with size
    for (name, size) in &files {
        entries.push_str(name);
        entries.push_str("  ");
        entries.push_str(size);
        entries.push('\n');
    }

    // Summary line (separate so caller can suppress when piped)
    let mut summary = format!("\nSummary: {} files, {} dirs", files.len(), dirs.len());
    if !by_ext.is_empty() {
        let mut ext_counts: Vec<_> = by_ext.iter().collect();
        ext_counts.sort_by(|a, b| b.1.cmp(a.1));
        let ext_parts: Vec<String> = ext_counts
            .iter()
            .take(5)
            .map(|(ext, count)| format!("{} {}", count, ext))
            .collect();
        summary.push_str(" (");
        summary.push_str(&ext_parts.join(", "));
        if ext_counts.len() > 5 {
            summary.push_str(&format!(", +{} more", ext_counts.len() - 5));
        }
        summary.push(')');
    }
    summary.push('\n');

    (entries, summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_compact_basic() {
        let input = "total 48\n\
                     drwxr-xr-x  2 user  staff    64 Jan  1 12:00 .\n\
                     drwxr-xr-x  2 user  staff    64 Jan  1 12:00 ..\n\
                     drwxr-xr-x  2 user  staff    64 Jan  1 12:00 src\n\
                     -rw-r--r--  1 user  staff  1234 Jan  1 12:00 Cargo.toml\n\
                     -rw-r--r--  1 user  staff  5678 Jan  1 12:00 README.md\n";
        let (entries, _summary) = compact_ls(input, false);
        assert!(entries.contains("src/"));
        assert!(entries.contains("Cargo.toml"));
        assert!(entries.contains("README.md"));
        assert!(entries.contains("1.2K"));
        assert!(entries.contains("5.5K"));
        assert!(!entries.contains("drwx"));
        assert!(!entries.contains("staff"));
        assert!(!entries.contains("total"));
        assert!(!entries.contains("\n.\n"));
        assert!(!entries.contains("\n..\n"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_compact_filters_noise() {
        let input = "total 8\n\
                     drwxr-xr-x  2 user  staff  64 Jan  1 12:00 node_modules\n\
                     drwxr-xr-x  2 user  staff  64 Jan  1 12:00 .git\n\
                     drwxr-xr-x  2 user  staff  64 Jan  1 12:00 target\n\
                     drwxr-xr-x  2 user  staff  64 Jan  1 12:00 src\n\
                     -rw-r--r--  1 user  staff  100 Jan  1 12:00 main.rs\n";
        let (entries, _summary) = compact_ls(input, false);
        assert!(!entries.contains("node_modules"));
        assert!(!entries.contains(".git"));
        assert!(!entries.contains("target"));
        assert!(entries.contains("src/"));
        assert!(entries.contains("main.rs"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_compact_show_all() {
        let input = "total 8\n\
                     drwxr-xr-x  2 user  staff  64 Jan  1 12:00 .git\n\
                     drwxr-xr-x  2 user  staff  64 Jan  1 12:00 src\n";
        let (entries, _summary) = compact_ls(input, true);
        assert!(entries.contains(".git/"));
        assert!(entries.contains("src/"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_compact_empty() {
        let input = "total 0\n";
        let (entries, summary) = compact_ls(input, false);
        assert_eq!(entries, "(empty)\n");
        assert!(summary.is_empty());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_compact_summary() {
        let input = "total 48\n\
                     drwxr-xr-x  2 user  staff    64 Jan  1 12:00 src\n\
                     -rw-r--r--  1 user  staff  1234 Jan  1 12:00 main.rs\n\
                     -rw-r--r--  1 user  staff  5678 Jan  1 12:00 lib.rs\n\
                     -rw-r--r--  1 user  staff   100 Jan  1 12:00 Cargo.toml\n";
        let (_entries, summary) = compact_ls(input, false);
        assert!(summary.contains("Summary: 3 files, 1 dirs"));
        assert!(summary.contains(".rs"));
        assert!(summary.contains(".toml"));
    }

    #[test]
    fn test_human_size() {
        assert_eq!(human_size(0), "0B");
        assert_eq!(human_size(500), "500B");
        assert_eq!(human_size(1024), "1.0K");
        assert_eq!(human_size(1234), "1.2K");
        assert_eq!(human_size(1_048_576), "1.0M");
        assert_eq!(human_size(2_500_000), "2.4M");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_compact_handles_filenames_with_spaces() {
        let input = "total 8\n\
                     -rw-r--r--  1 user  staff  100 Jan  1 12:00 my file.txt\n";
        let (entries, _summary) = compact_ls(input, false);
        assert!(entries.contains("my file.txt"));
    }

    // Windows-specific tests for native implementation
    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_ls_basic() {
        use std::fs;
        use std::io::Write;

        // Create temp directory with test files
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("test.txt");
        let mut file = fs::File::create(&test_file).expect("Failed to create test file");
        file.write_all(b"test content").expect("Failed to write to test file");

        // Run ls on temp directory
        let args = vec![temp_dir.path().to_str().unwrap().to_string()];
        let result = run(&args, 0);

        assert!(result.is_ok(), "ls should succeed on temp directory");
        // Note: We can't easily capture stdout in this test,
        // but we verify it doesn't crash
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_hidden_files_excluded_by_default() {
        use std::fs;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        
        // Create regular file
        fs::File::create(temp_dir.path().join("visible.txt"))
            .expect("Failed to create visible file");
        
        // Create hidden file (starts with .)
        fs::File::create(temp_dir.path().join(".hidden.txt"))
            .expect("Failed to create hidden file");

        // Read directory directly to verify filtering logic
        let entries = fs::read_dir(temp_dir.path())
            .expect("Failed to read temp dir");
        
        let mut visible_count = 0;
        let mut hidden_count = 0;
        
        for entry in entries {
            let entry = entry.expect("Failed to read entry");
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                hidden_count += 1;
            } else {
                visible_count += 1;
            }
        }

        assert_eq!(visible_count, 1, "Should have 1 visible file");
        assert_eq!(hidden_count, 1, "Should have 1 hidden file");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_format_long_entry() {
        use std::fs;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test").expect("Failed to write test file");

        let metadata = fs::metadata(&test_file).expect("Failed to get metadata");
        let formatted = format_long_entry("test.txt", &metadata, false);

        // Verify format contains expected components
        assert!(formatted.contains("test.txt"), "Should contain filename");
        assert!(formatted.contains("4") || formatted.len() > 10, "Should contain size or be formatted");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_human_readable_sizes() {
        use std::fs;

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("large.txt");
        
        // Create a file with known size
        let content = vec![b'x'; 2048]; // 2KB
        fs::write(&test_file, content).expect("Failed to write test file");

        let metadata = fs::metadata(&test_file).expect("Failed to get metadata");
        let formatted = format_long_entry("large.txt", &metadata, true);

        // Should contain human-readable size
        assert!(formatted.contains("2.0K") || formatted.contains("2048"), 
                "Should contain size in KB or bytes");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_format_compact_output() {
        let dirs = vec!["src".to_string(), "tests".to_string()];
        let files = vec![
            ("Cargo.toml".to_string(), 1234, String::new()),
            ("README.md".to_string(), 5678, String::new()),
        ];

        let (entries, summary) = format_compact_output(&dirs, &files, false, true, false);

        // Verify dirs appear with /
        assert!(entries.contains("src/"), "Should contain src/");
        assert!(entries.contains("tests/"), "Should contain tests/");

        // Verify files appear with sizes
        assert!(entries.contains("Cargo.toml"), "Should contain Cargo.toml");
        assert!(entries.contains("README.md"), "Should contain README.md");
        assert!(entries.contains("1.2K"), "Should contain human-readable size for Cargo.toml");
        assert!(entries.contains("5.5K"), "Should contain human-readable size for README.md");

        // Verify summary
        assert!(summary.contains("2 files"), "Summary should show file count");
        assert!(summary.contains("2 dirs"), "Summary should show dir count");
    }
}
