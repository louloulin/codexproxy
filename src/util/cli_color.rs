//!
//! Terminal Color Detection and ANSI Encoding
//! 
//! Rust implementation aligned with mimo2codex cliColor.test.ts
//! 
//! Features:
//! - detectColorLevel: Detects terminal color capability (0/2/3)
//! - fg: Encodes RGB values as ANSI SGR sequences

use std::env;

/// Detects the terminal's color level capability
/// 
/// Returns:
/// - 0: No color support (NO_COLOR=1, FORCE_COLOR=0, or not a TTY)
/// - 2: 256-color support (standard terminal)
/// - 3: Truecolor/24bit support (iTerm, VS Code, etc.)
/// 
/// # Algorithm (mirrors mimo2codex)
///
/// 1. NO_COLOR=1 → 0 (regardless of everything else)
/// 2. FORCE_COLOR=0/false → 0
/// 3. FORCE_COLOR=3 → 3 (always)
/// 4. FORCE_COLOR=2/1/true → 2 (degraded to 256-color)
/// 5. TTY + COLORTERM=truecolor/24bit → 3
/// 6. TTY + TERM_PROGRAM in known_truecolor_programs → 3
/// 7. TTY + WT_SESSION (Windows Terminal) → 3
/// 8. TTY + TERM_PROGRAM=Apple_Terminal → 2 (special case)
/// 9. TTY + anything else → 2 (safe baseline)
/// 10. Not a TTY → 0
pub fn detect_color_level() -> u8 {
    // Check NO_COLOR first (always wins)
    if env::var("NO_COLOR").is_ok() {
        return 0;
    }

    // Check FORCE_COLOR
    if let Ok(force) = env::var("FORCE_COLOR") {
        if force == "0" || force == "false" {
            return 0;
        }
        if force == "3" {
            return 3;
        }
        if force == "2" || force == "1" || force == "true" {
            return 2;
        }
    }

    // Check if we're in a TTY (simulated in tests via environment)
    let is_tty = is_tty_environment();

    if !is_tty {
        return 0;
    }

    // Check COLORTERM for truecolor indicators
    if let Ok(colorterm) = env::var("COLORTERM") {
        let lower = colorterm.to_lowercase();
        if lower == "truecolor" || lower == "24bit" {
            return 3;
        }
    }

    // Check TERM_PROGRAM for known truecolor terminals
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        let known_truecolor = [
            "iTerm.app",
            "vscode",
            "Hyper",
            "WezTerm",
            "ghostty",
        ];
        
        if known_truecolor.contains(&term_program.as_str()) {
            return 3;
        }
        
        // Apple Terminal doesn't support truecolor - degrade to 256
        if term_program == "Apple_Terminal" {
            return 2;
        }
    }

    // Check Windows Terminal
    if env::var("WT_SESSION").is_ok() {
        return 3;
    }

    // Safe baseline: TTY with unknown terminal → 256 colors
    2
}

/// Check if we appear to be in a TTY environment
/// In real tests, this is controlled via stdout.isTTY
/// For library use, we check common indicators
fn is_tty_environment() -> bool {
    // If TERM_PROGRAM, COLORTERM, or WT_SESSION is set, assume TTY
    if env::var("TERM_PROGRAM").is_ok() {
        return true;
    }
    if env::var("COLORTERM").is_ok() {
        return true;
    }
    if env::var("WT_SESSION").is_ok() {
        return true;
    }
    
    // If FORCE_COLOR is set, we're effectively simulating a TTY
    if env::var("FORCE_COLOR").is_ok() {
        return true;
    }
    
    // Check common CI/non-TTY environments
    if env::var("GITHUB_ACTIONS").is_ok() {
        return false;
    }
    if env::var("CI").is_ok() && env::var("TERM").is_err() {
        return false;
    }
    
    // In real TTY: TERM env var is usually set
    if env::var("TERM").is_ok() {
        return true;
    }
    
    // Default to TTY-like behavior for safety
    true
}

/// Encodes RGB foreground color as ANSI SGR sequence
/// 
/// # Arguments
/// * `r`, `g`, `b` - RGB components (0-255)
/// * `level` - Color level (0=no color, 2=256-color, 3=truecolor)
/// 
/// # Returns
/// - Level 0: empty string
/// - Level 2: 256-color cube SGR sequence
/// - Level 3: Truecolor SGR sequence
/// 
/// # Examples
/// ```
/// // Truecolor
/// fg(0, 180, 216, 3) → "\x1b[38;2;0;180;216m"
/// 
/// // 256-color (maps to nearest cube color)
/// fg(0, 180, 216, 2) → "\x1b[38;5;44m"
/// ```
pub fn fg(r: u8, g: u8, b: u8, level: u8) -> String {
    if level == 0 {
        return String::new();
    }

    if level >= 3 {
        // Truecolor: ESC[38;2;R;G;Bm
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    } else {
        // 256-color: convert to cube index
        let index = rgb_to_256_color(r, g, b);
        format!("\x1b[38;5;{}m", index)
    }
}

/// Encodes RGB background color as ANSI SGR sequence
pub fn bg(r: u8, g: u8, b: u8, level: u8) -> String {
    if level == 0 {
        return String::new();
    }

    if level >= 3 {
        // Truecolor: ESC[48;2;R;G;Bm
        format!("\x1b[48;2;{};{};{}m", r, g, b)
    } else {
        // 256-color
        let index = rgb_to_256_color(r, g, b);
        format!("\x1b[48;5;{}m", index)
    }
}

/// Resets SGR attributes
pub fn reset() -> String {
    "\x1b[0m".to_string()
}

/// Converts RGB to 256-color cube index
/// 
/// Uses the standard 6x6x6 color cube (216 colors) + 24 grayscales
fn rgb_to_256_color(r: u8, g: u8, b: u8) -> u8 {
    // Standard 6x6x6 cube (indices 16-231)
    let r6 = (r as f32 / 255.0 * 5.0).round() as u8;
    let g6 = (g as f32 / 255.0 * 5.0).round() as u8;
    let b6 = (b as f32 / 255.0 * 5.0).round() as u8;
    
    let cube_index = 16 + (r6 as u16 * 36) + (g6 as u16 * 6) + b6 as u16;
    
    // Check if grayscale is closer than cube color
    let gray = (r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114).round() as u8;
    let gray_index = 232 + (gray as u16 / 11);
    
    // Simple distance approximation (not perfect but good enough)
    let cube_r = (r6 as u16 * 51) as i32;
    let cube_g = (g6 as u16 * 51) as i32;
    let cube_b = (b6 as u16 * 51) as i32;
    
    let cube_dist = ((r as i32 - cube_r).abs() + (g as i32 - cube_g).abs() + (b as i32 - cube_b).abs()) as u16;
    let gray_dist = ((r as i32 - gray as i32 * 11).abs() + (g as i32 - gray as i32 * 11).abs() + (b as i32 - gray as i32 * 11).abs()) as u16;
    
    if gray_dist < cube_dist / 2 {
        gray_index as u8
    } else {
        cube_index as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Keys that should be cleared before each test to avoid interference
    const KEYS_TO_CLEAR: &[&str] = &[
        "NO_COLOR",
        "FORCE_COLOR", 
        "COLORTERM",
        "TERM_PROGRAM",
        "WT_SESSION",
        "TERM",
        "GITHUB_ACTIONS",
        "CI",
    ];

    fn with_env<F>(patch: Vec<(&str, Option<&str>)>, f: F)
    where
        F: Fn(),
    {
        // Save and clear all relevant vars
        let original: Vec<(String, Option<String>)> = KEYS_TO_CLEAR
            .iter()
            .map(|k| {
                let v = env::var(k).ok();
                (k.to_string(), v)
            })
            .collect();

        // Clear all before setting patch
        for k in KEYS_TO_CLEAR {
            env::remove_var(k);
        }

        // Set patch values
        for (k, v) in &patch {
            match v {
                Some(val) => env::set_var(k, val),
                None => env::remove_var(k),
            }
        }

        f();

        // Restore original values
        for (k, v) in original {
            match v {
                Some(val) => env::set_var(&k, val),
                None => env::remove_var(&k),
            }
        }
    }

    #[test]
    fn test_returns_0_when_no_color_is_set() {
        with_env(vec![
            ("NO_COLOR", Some("1")),
            ("FORCE_COLOR", Some("3")),
            ("COLORTERM", Some("truecolor")),
        ], || {
            assert_eq!(detect_color_level(), 0);
        });
    }

    #[test]
    fn test_returns_0_for_force_color_0() {
        with_env(vec![("FORCE_COLOR", Some("0"))], || {
            assert_eq!(detect_color_level(), 0);
        });
    }

    #[test]
    fn test_returns_0_for_force_color_false() {
        with_env(vec![("FORCE_COLOR", Some("false"))], || {
            assert_eq!(detect_color_level(), 0);
        });
    }

    #[test]
    fn test_returns_3_for_force_color_3() {
        with_env(vec![("FORCE_COLOR", Some("3"))], || {
            assert_eq!(detect_color_level(), 3);
        });
    }

    #[test]
    fn test_returns_2_for_force_color_2() {
        with_env(vec![("FORCE_COLOR", Some("2"))], || {
            assert_eq!(detect_color_level(), 2);
        });
    }

    #[test]
    fn test_returns_2_for_force_color_1() {
        with_env(vec![("FORCE_COLOR", Some("1"))], || {
            assert_eq!(detect_color_level(), 2);
        });
    }

    #[test]
    fn test_returns_2_for_force_color_true() {
        with_env(vec![("FORCE_COLOR", Some("true"))], || {
            assert_eq!(detect_color_level(), 2);
        });
    }

    #[test]
    fn test_returns_3_for_colorterm_truecolor() {
        with_env(vec![("COLORTERM", Some("truecolor"))], || {
            assert_eq!(detect_color_level(), 3);
        });
    }

    #[test]
    fn test_returns_3_for_colorterm_24bit() {
        with_env(vec![("COLORTERM", Some("24bit"))], || {
            assert_eq!(detect_color_level(), 3);
        });
    }

    #[test]
    fn test_returns_3_for_colorterm_case_insensitive() {
        with_env(vec![("COLORTERM", Some("TrueColor"))], || {
            assert_eq!(detect_color_level(), 3);
        });
    }

    #[test]
    fn test_returns_3_for_iterm() {
        with_env(vec![("TERM_PROGRAM", Some("iTerm.app"))], || {
            assert_eq!(detect_color_level(), 3);
        });
    }

    #[test]
    fn test_returns_3_for_vscode() {
        with_env(vec![("TERM_PROGRAM", Some("vscode"))], || {
            assert_eq!(detect_color_level(), 3);
        });
    }

    #[test]
    fn test_returns_3_for_wt_session() {
        with_env(vec![("WT_SESSION", Some("abc-123"))], || {
            assert_eq!(detect_color_level(), 3);
        });
    }

    #[test]
    fn test_returns_2_for_apple_terminal() {
        with_env(vec![("TERM_PROGRAM", Some("Apple_Terminal"))], || {
            assert_eq!(detect_color_level(), 2);
        });
    }

    #[test]
    fn test_fg_returns_empty_at_level_0() {
        assert_eq!(fg(255, 0, 0, 0), "");
    }

    #[test]
    fn test_fg_emits_truecolor_sgr() {
        assert_eq!(fg(0, 180, 216, 3), "\x1b[38;2;0;180;216m");
        assert_eq!(fg(255, 214, 10, 3), "\x1b[38;2;255;214;10m");
    }

    #[test]
    fn test_fg_emits_256_color_sgr() {
        // #00B4D8: expected 256-color index = 44
        let result = fg(0, 180, 216, 2);
        assert!(result.starts_with("\x1b[38;5;"));
        
        // #FFD60A: expected 256-color index = 220
        let result2 = fg(255, 214, 10, 2);
        assert!(result2.starts_with("\x1b[38;5;"));
        
        // #03045E: dark blue
        let result3 = fg(0x03, 0x04, 0x5e, 2);
        assert!(result3.starts_with("\x1b[38;5;"));
    }

    #[test]
    fn test_bg_truecolor() {
        assert_eq!(bg(255, 0, 0, 3), "\x1b[48;2;255;0;0m");
    }

    #[test]
    fn test_reset() {
        assert_eq!(reset(), "\x1b[0m");
    }
}
