//!
//! Update Method Detection Module
//! 
//! Rust implementation aligned with mimo2codex updateMethod.test.ts
//! 
//! Features:
//! - Detect update method (git vs npm)
//! - Build update command
//! - Package root detection

use std::path::PathBuf;
use std::process::Command;

/// Update method enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateMethod {
    /// Update via git pull
    Git,
    /// Update via npm/yarn install
    Npm,
    /// No update method available
    None,
}

/// Update information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// The detected update method
    pub method: UpdateMethod,
    /// Steps to perform the update
    pub steps: Vec<UpdateStep>,
    /// Root directory of the package
    pub root_dir: PathBuf,
    /// Full command string
    pub command: String,
}

/// A single update step
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateStep {
    /// Command to execute
    pub argv: Vec<String>,
    /// Description of the step
    pub description: String,
}

/// Get the package root directory
/// First checks for a Cargo.toml (Rust) or package.json (Node.js)
pub fn package_root() -> PathBuf {
    // Start from current working directory
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    loop {
        // Check for Rust package
        if dir.join("Cargo.toml").exists() {
            return dir;
        }
        // Check for Node.js package
        if dir.join("package.json").exists() {
            return dir;
        }
        
        // Move up one directory
        if !dir.pop() {
            // Reached filesystem root, use current directory
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

/// Check if we're in a git repository
pub fn is_git_repo() -> bool {
    let root = package_root();
    Command::new("git")
        .args(["-C", root.to_string_lossy().as_ref(), "rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect the update method based on the environment
pub fn detect_update_method() -> UpdateInfo {
    let root_dir = package_root();
    let method = if is_git_repo() {
        UpdateMethod::Git
    } else if root_dir.join("package.json").exists() {
        UpdateMethod::Npm
    } else {
        UpdateMethod::None
    };
    
    let (steps, command) = match method {
        UpdateMethod::Git => {
            let steps = vec![
                UpdateStep {
                    argv: vec!["git".to_string(), "pull".to_string(), "--ff-only".to_string()],
                    description: "Pull latest changes from git".to_string(),
                },
                UpdateStep {
                    argv: vec!["cargo".to_string(), "build".to_string(), "--release".to_string()],
                    description: "Build the project".to_string(),
                },
            ];
            let command = format!(
                "git -C {} pull --ff-only && cargo build --release",
                root_dir.to_string_lossy()
            );
            (steps, command)
        }
        UpdateMethod::Npm => {
            let steps = vec![
                UpdateStep {
                    argv: vec!["npm".to_string(), "install".to_string()],
                    description: "Install npm dependencies".to_string(),
                },
                UpdateStep {
                    argv: vec!["npm".to_string(), "run".to_string(), "build".to_string()],
                    description: "Build the project".to_string(),
                },
            ];
            let command = "npm install && npm run build".to_string();
            (steps, command)
        }
        UpdateMethod::None => {
            let steps = vec![];
            let command = String::new();
            (steps, command)
        }
    };
    
    UpdateInfo {
        method,
        steps,
        root_dir,
        command,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_root_finds_cargo_toml() {
        let root = package_root();
        // Should find a directory with Cargo.toml or package.json
        assert!(
            root.join("Cargo.toml").exists() || root.join("package.json").exists(),
            "package_root should find a valid package directory"
        );
    }

    #[test]
    fn test_detect_update_method_returns_non_empty_command() {
        let info = detect_update_method();
        // Method should be valid
        assert_ne!(info.method, UpdateMethod::None);
        // Root dir should be set
        assert!(info.root_dir.exists());
    }

    #[test]
    fn test_update_info_has_steps() {
        let info = detect_update_method();
        if info.method == UpdateMethod::None {
            return; // Skip if no update method available
        }
        assert!(!info.steps.is_empty(), "Update steps should not be empty");
    }

    #[test]
    fn test_update_step_has_command() {
        let info = detect_update_method();
        if info.method == UpdateMethod::None {
            return;
        }
        for step in &info.steps {
            assert!(!step.argv.is_empty(), "Each step should have a command");
        }
    }

    #[test]
    fn test_git_or_npm_available() {
        // Check if git or npm is available
        let git_available = Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        let npm_available = Command::new("npm")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        // At least one should be available
        assert!(git_available || npm_available, "Either git or npm should be available");
    }

    #[test]
    fn test_update_info_command_format() {
        let info = detect_update_method();
        if info.method == UpdateMethod::None {
            return;
        }
        
        match info.method {
            UpdateMethod::Git => {
                assert!(info.command.contains("git"));
                assert!(info.command.contains("pull"));
            }
            UpdateMethod::Npm => {
                assert!(info.command.contains("npm"));
            }
            UpdateMethod::None => {}
        }
    }
}
