//! Hosts file management for localhost subdomain resolution.
//!
//! Windows doesn't resolve `*.localhost` subdomains by default.
//! This module adds the necessary entries to the hosts file.
//!
//! Note: This requires the application to run with administrator privileges.

use std::fs::{self, OpenOptions};
use std::io::Write;

const HOSTS_MARKER_START: &str = "# BEGIN rai-connect";
const HOSTS_MARKER_END: &str = "# END rai-connect";

const LOCALHOST_ENTRIES: &[(&str, &str)] = &[
    ("127.0.0.1", "osu.localhost"),
    ("127.0.0.1", "c.localhost"),
    ("127.0.0.1", "a.localhost"),
    ("127.0.0.1", "b.localhost"),
    ("127.0.0.1", "i.localhost"),
];

#[cfg(target_os = "windows")]
const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

#[cfg(not(target_os = "windows"))]
const HOSTS_PATH: &str = "/etc/hosts";

/// Checks if the rai-connect hosts entries are already present.
pub fn are_hosts_entries_present() -> bool {
    match fs::read_to_string(HOSTS_PATH) {
        Ok(content) => content.contains(HOSTS_MARKER_START),
        Err(_) => false,
    }
}

/// Generates the hosts file content block for rai-connect.
fn generate_hosts_block() -> String {
    let mut block = String::new();
    block.push_str(HOSTS_MARKER_START);
    block.push('\n');
    for (ip, hostname) in LOCALHOST_ENTRIES {
        block.push_str(&format!("{} {}\n", ip, hostname));
    }
    block.push_str(HOSTS_MARKER_END);
    block
}

/// Adds localhost subdomain entries to the hosts file.
///
/// This requires administrator privileges. The application should be
/// run as admin (via Windows manifest) for this to work.
///
/// # Returns
///
/// Returns `Ok(true)` if entries were added, `Ok(false)` if they already exist,
/// or an error if the operation failed.
pub fn add_hosts_entries() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if are_hosts_entries_present() {
        tracing::info!("Hosts entries already present");
        return Ok(false);
    }

    let block = generate_hosts_block();

    // Append the block to the hosts file
    let mut file = OpenOptions::new()
        .append(true)
        .open(HOSTS_PATH)
        .map_err(|e| format!("Failed to open hosts file: {}. Make sure the app is running as administrator.", e))?;

    // Add a newline before our block if the file doesn't end with one
    let content = fs::read_to_string(HOSTS_PATH)?;
    let prefix = if content.ends_with('\n') { "" } else { "\n" };

    file.write_all(format!("{}{}\n", prefix, block).as_bytes())
        .map_err(|e| format!("Failed to write to hosts file: {}", e))?;

    // Verify the entries were added
    if are_hosts_entries_present() {
        tracing::info!("Successfully added hosts entries");
        Ok(true)
    } else {
        Err("Failed to verify hosts entries were added".into())
    }
}

/// Removes rai-connect entries from the hosts file.
///
/// This requires administrator privileges.
pub fn remove_hosts_entries() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if !are_hosts_entries_present() {
        return Ok(false);
    }

    let content = fs::read_to_string(HOSTS_PATH)?;

    // Find and remove the rai-connect block
    let start_idx = content.find(HOSTS_MARKER_START);
    let end_idx = content.find(HOSTS_MARKER_END);

    if let (Some(start), Some(end)) = (start_idx, end_idx) {
        // Find the start of the line containing the marker
        let line_start = content[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        // Find the end of the line containing the end marker
        let line_end = content[end..].find('\n').map(|i| end + i + 1).unwrap_or(content.len());

        let mut new_content = String::new();
        new_content.push_str(&content[..line_start]);
        new_content.push_str(&content[line_end..]);

        // Remove any double newlines that might result
        let new_content = new_content.replace("\n\n\n", "\n\n");

        fs::write(HOSTS_PATH, new_content)
            .map_err(|e| format!("Failed to write hosts file: {}", e))?;

        tracing::info!("Successfully removed hosts entries");
        Ok(true)
    } else {
        Err("Failed to find hosts block boundaries".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hosts_block() {
        let block = generate_hosts_block();
        assert!(block.contains(HOSTS_MARKER_START));
        assert!(block.contains(HOSTS_MARKER_END));
        assert!(block.contains("osu.localhost"));
        assert!(block.contains("c.localhost"));
    }
}
