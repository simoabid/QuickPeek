use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn get_state_dir() -> PathBuf {
    if let Some(xdg) = env::var_os("XDG_STATE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("quickpeek");
        }
    }
    if let Some(home) = env::var_os("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".local/state/quickpeek");
        }
    }
    PathBuf::from(".local/state/quickpeek")
}

#[allow(dead_code)]
pub fn get_state_file_path() -> PathBuf {
    get_state_dir().join("last_selection")
}

pub fn load_last_selection_from(state_dir: &Path) -> Option<PathBuf> {
    let state_file = state_dir.join("last_selection");
    let content = fs::read_to_string(&state_file).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

pub fn load_last_selection() -> Option<PathBuf> {
    load_last_selection_from(&get_state_dir())
}

pub fn save_last_selection_in(state_dir: &Path, path: &Path) -> Result<(), io::Error> {
    if !state_dir.exists() {
        fs::create_dir_all(state_dir)?;
    }
    let _ = fs::set_permissions(state_dir, fs::Permissions::from_mode(0o700));

    let temp_name = format!(
        ".last_selection.tmp.{}.{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let temp_path = state_dir.join(temp_name);
    let target_path = state_dir.join("last_selection");

    let content = format!("{}\n", path.display());
    fs::write(&temp_path, content.as_bytes())?;
    let _ = fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600));

    if let Err(e) = fs::rename(&temp_path, &target_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }

    Ok(())
}

pub fn save_last_selection(path: &Path) -> Result<(), io::Error> {
    save_last_selection_in(&get_state_dir(), path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("qp_state_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);

        let test_path = PathBuf::from("/sample/path/image.png");
        assert!(save_last_selection_in(&tmp, &test_path).is_ok());

        let loaded = load_last_selection_from(&tmp);
        assert_eq!(loaded, Some(test_path));

        // Verify directory mode is 0700
        let meta = fs::metadata(&tmp).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o700);

        // Verify file content has trailing newline
        let raw = fs::read_to_string(tmp.join("last_selection")).unwrap();
        assert_eq!(raw, "/sample/path/image.png\n");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_nonexistent_state_file() {
        let tmp = std::env::temp_dir().join(format!("qp_state_test_missing_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);

        assert_eq!(load_last_selection_from(&tmp), None);
    }

    #[test]
    fn test_load_empty_or_whitespace_state_file() {
        let tmp = std::env::temp_dir().join(format!("qp_state_test_empty_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);

        fs::write(tmp.join("last_selection"), b"   \n  \n").unwrap();
        assert_eq!(load_last_selection_from(&tmp), None);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_atomic_overwrite() {
        let tmp = std::env::temp_dir().join(format!("qp_state_test_overwrite_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);

        let path1 = PathBuf::from("/first/image.png");
        let path2 = PathBuf::from("/second/image.png");

        assert!(save_last_selection_in(&tmp, &path1).is_ok());
        assert_eq!(load_last_selection_from(&tmp), Some(path1));

        assert!(save_last_selection_in(&tmp, &path2).is_ok());
        assert_eq!(load_last_selection_from(&tmp), Some(path2));

        let _ = fs::remove_dir_all(&tmp);
    }
}
