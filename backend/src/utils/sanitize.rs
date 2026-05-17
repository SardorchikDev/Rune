//! Shell argument sanitisation for the [`crate::tools::terminal`] tool.

/// Patterns that are always rejected before a command reaches `bash -c`.
/// Each entry is treated as a case-insensitive substring match: if any of
/// these appears anywhere in the command string we refuse to spawn the
/// child process at all.
pub const BLOCKLIST_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf --no-preserve-root",
    ":(){",      // fork-bomb prefix
    "/dev/sd",   // raw disks
    "/dev/nvme", // raw NVMe disks
    "mkfs",
    "shutdown",
    "reboot",
    "init 0",
    "init 6",
    "poweroff",
    "halt",
    "sudo",
    "su -",
    "su root",
    "passwd",
];

/// Result type for sanitisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SanitizeOutcome {
    /// Command is safe to execute.
    Ok,
    /// Command was rejected because it matched the blocklist pattern.
    Rejected(&'static str),
}

/// Inspects the given command and returns whether it is safe to spawn.
pub fn check_command(command: &str) -> SanitizeOutcome {
    let lowered = command.to_lowercase();
    for pattern in BLOCKLIST_PATTERNS {
        if lowered.contains(&pattern.to_lowercase()) {
            return SanitizeOutcome::Rejected(pattern);
        }
    }
    SanitizeOutcome::Ok
}

/// Returns `true` if the given command is allowed.
pub fn is_allowed(command: &str) -> bool {
    matches!(check_command(command), SanitizeOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_rm_rf_root() {
        assert!(!is_allowed("rm -rf /"));
        assert!(!is_allowed("RM -RF /"));
    }

    #[test]
    fn rejects_fork_bomb() {
        assert!(!is_allowed(":(){ :|:&; };:"));
    }

    #[test]
    fn rejects_sudo() {
        assert!(!is_allowed("sudo apt-get update"));
    }

    #[test]
    fn allows_normal_commands() {
        assert!(is_allowed("ls -la"));
        assert!(is_allowed("echo hello"));
        assert!(is_allowed("cargo build --release"));
    }
}
