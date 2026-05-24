// Temporary test file to verify the fix
use tempfile::TempDir;
use std::env;

#[test]
fn test_apply_codex_creates_files() {
    // Declare temp_dir FIRST so it's dropped LAST
    let temp_dir = TempDir::new().unwrap();
    let codex_path = temp_dir.path().join("m2c-test-codex");
    env::set_var("CODEX_HOME", codex_path.to_string_lossy().as_ref());
    
    // Do the work...
    
    // Remove CODEX_HOME BEFORE temp_dir is dropped
    env::remove_var("CODEX_HOME");
    
    // Now temp_dir can be dropped safely after assertions are done
    // The `_ = temp_dir` trick won't help because Rust drops in reverse order
}
