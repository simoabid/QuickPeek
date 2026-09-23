use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

// AT-SPI Role IDs (matching standard atspi-constants.h)
#[allow(dead_code)]
pub const ROLE_CHECK_BOX: u32 = 7;
pub const ROLE_COMBO_BOX: u32 = 11;
pub const ROLE_DIALOG: u32 = 16;
pub const ROLE_FRAME: u32 = 23;
pub const ROLE_LIST: u32 = 31;
#[allow(dead_code)]
pub const ROLE_LIST_ITEM: u32 = 32;
pub const ROLE_MENU_BAR: u32 = 34;
pub const ROLE_MENU_ITEM: u32 = 35;
pub const ROLE_PUSH_BUTTON: u32 = 43;
pub const ROLE_RADIO_BUTTON: u32 = 44;
pub const ROLE_SCROLL_BAR: u32 = 54;
pub const ROLE_SEPARATOR: u32 = 56;
pub const ROLE_STATUS_BAR: u32 = 59;
pub const ROLE_TABLE: u32 = 60;
#[allow(dead_code)]
pub const ROLE_TABLE_CELL: u32 = 61;
pub const ROLE_TOOL_BAR: u32 = 67;
pub const ROLE_TREE_TABLE: u32 = 69;
pub const ROLE_WINDOW: u32 = 74;

// AT-SPI State Bits
// Bit 1: ATSPI_STATE_ACTIVE (1 << 1 == 2)
pub const STATE_ACTIVE_MASK: u32 = 2;

/// Checks if a given role should be pruned during tree descent to avoid
/// traversing large irrelevant subtrees (e.g. menus, toolbars, buttons).
pub fn should_prune_role(role: u32) -> bool {
    matches!(
        role,
        ROLE_MENU_BAR
            | ROLE_MENU_ITEM
            | ROLE_TOOL_BAR
            | ROLE_STATUS_BAR
            | ROLE_SCROLL_BAR
            | ROLE_SEPARATOR
            | ROLE_PUSH_BUTTON
            | ROLE_CHECK_BOX
            | ROLE_RADIO_BUTTON
    )
}

/// Checks if a role represents a selectable container view in Dolphin.
pub fn is_view_role(role: u32) -> bool {
    matches!(role, ROLE_LIST | ROLE_TABLE | ROLE_TREE_TABLE)
}

/// Checks if a role and state vector indicate an active, focused top-level window.
pub fn is_active_window(state: &[u32], role: u32) -> bool {
    let is_window_role = matches!(role, ROLE_FRAME | ROLE_WINDOW | ROLE_DIALOG);
    let has_active_bit = state.first().map(|&s| (s & STATE_ACTIVE_MASK) != 0).unwrap_or(false);
    is_window_role && has_active_bit
}

/// Parses a directory path from a Dolphin window title caption.
///
/// Dolphin captions typically end with " — Dolphin", " - Dolphin", or " – Dolphin".
/// This function strips known suffixes and validates if the result is an absolute directory.
pub fn parse_dir_from_window_title(title: &str) -> Option<PathBuf> {
    let trimmed = title.trim();
    let cleaned = if let Some(stripped) = trimmed.strip_suffix(" — Dolphin") {
        stripped.trim()
    } else if let Some(stripped) = trimmed.strip_suffix(" - Dolphin") {
        stripped.trim()
    } else if let Some(stripped) = trimmed.strip_suffix(" – Dolphin") {
        stripped.trim()
    } else {
        trimmed
    };

    if cleaned.is_empty() {
        return None;
    }

    let candidate = if cleaned.starts_with("~/") || cleaned == "~" {
        if let Ok(home) = std::env::var("HOME") {
            if cleaned == "~" {
                PathBuf::from(home)
            } else {
                PathBuf::from(home).join(&cleaned[2..])
            }
        } else {
            PathBuf::from(cleaned)
        }
    } else {
        PathBuf::from(cleaned)
    };

    if candidate.is_absolute() && candidate.is_dir() {
        Some(candidate)
    } else {
        None
    }
}

/// Derives the absolute path of a selected file by matching against candidate directories.
pub fn derive_selected_path(candidate_dirs: &[PathBuf], filename: &str) -> Result<PathBuf, String> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return Err("selected item has empty filename".into());
    }

    let direct_path = Path::new(trimmed);
    if direct_path.is_absolute() && direct_path.is_file() {
        return Ok(direct_path.to_path_buf());
    }

    for dir in candidate_dirs {
        let candidate = dir.join(trimmed);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    // Secondary fallback: if candidate dir exists, join even if file not yet verified
    if let Some(first_dir) = candidate_dirs.first() {
        return Ok(first_dir.join(trimmed));
    }

    Err(format!(
        "could not derive directory path for selected file '{}'",
        trimmed
    ))
}

/// Helper to verify deadline is not exceeded.
pub fn check_deadline(deadline: Instant, step: &str) -> Result<(), String> {
    if Instant::now() >= deadline {
        Err(format!("AT-SPI deadline exceeded at {}", step))
    } else {
        Ok(())
    }
}

/// Retrieves the AT-SPI bus address from the session bus via `org.a11y.Bus.GetAddress`.
pub fn get_a11y_address(session_conn: &gio::DBusConnection, timeout_ms: i32) -> Result<String, String> {
    let reply = session_conn
        .call_sync(
            Some("org.a11y.Bus"),
            "/org/a11y/bus",
            "org.a11y.Bus",
            "GetAddress",
            None,
            None,
            gio::DBusCallFlags::NO_AUTO_START,
            timeout_ms,
            gio::Cancellable::NONE,
        )
        .map_err(|e| format!("org.a11y.Bus GetAddress failed: {}", e))?;

    let addr: String = reply.child_get(0);
    if addr.is_empty() {
        Err("empty AT-SPI bus address returned".into())
    } else {
        Ok(addr)
    }
}

/// Connects to the AT-SPI accessibility message bus using raw gio D-Bus.
pub fn connect_a11y_bus(addr: &str) -> Result<gio::DBusConnection, String> {
    let flags =
        gio::DBusConnectionFlags::AUTHENTICATION_CLIENT | gio::DBusConnectionFlags::MESSAGE_BUS_CONNECTION;
    gio::DBusConnection::for_address_sync(addr, flags, None::<&gio::DBusAuthObserver>, gio::Cancellable::NONE)
        .map_err(|e| format!("failed to connect to AT-SPI bus at '{}': {}", addr, e))
}

/// Resolves the currently selected file in the active Dolphin window via the AT-SPI bus.
///
/// Bounded by the specified deadline (150 ms maximum).
pub fn resolve_dolphin_selection(
    session_conn: &gio::DBusConnection,
    deadline: Instant,
) -> Result<PathBuf, String> {
    check_deadline(deadline, "start")?;

    // 1. Discover a11y address on session bus
    let a11y_addr = get_a11y_address(session_conn, 30)?;
    check_deadline(deadline, "address discovery")?;

    // 2. Connect to a11y bus
    let a11y_conn = connect_a11y_bus(&a11y_addr)?;
    check_deadline(deadline, "bus connection")?;

    // 3. Get desktop children from root
    let root_reply = a11y_conn
        .call_sync(
            Some("org.a11y.atspi.Registry"),
            "/org/a11y/atspi/accessible/root",
            "org.a11y.atspi.Accessible",
            "GetChildren",
            None,
            None,
            gio::DBusCallFlags::NO_AUTO_START,
            30,
            gio::Cancellable::NONE,
        )
        .map_err(|e| format!("AT-SPI root GetChildren failed: {}", e))?;

    let children_variant = root_reply.child_value(0);

    // 4. Find Dolphin application among desktop children
    let mut dolphin_bus = None;
    let mut dolphin_path = None;

    for i in 0..children_variant.n_children() {
        check_deadline(deadline, "app search")?;
        let child = children_variant.child_value(i);
        let bus_name: String = child.child_get(0);
        let path: String = child.child_get(1);

        let name = get_accessible_name(&a11y_conn, &bus_name, &path, 20);
        if name.eq_ignore_ascii_case("dolphin") {
            dolphin_bus = Some(bus_name);
            dolphin_path = Some(path);
            break;
        }
    }

    let (bus, app_path) = match (dolphin_bus, dolphin_path) {
        (Some(b), Some(p)) => (b, p),
        _ => return Err("Dolphin application not found on accessibility bus".into()),
    };

    // 5. Inspect Dolphin's windows
    check_deadline(deadline, "window search")?;
    let win_reply = a11y_conn
        .call_sync(
            Some(&bus),
            &app_path,
            "org.a11y.atspi.Accessible",
            "GetChildren",
            None,
            None,
            gio::DBusCallFlags::NO_AUTO_START,
            30,
            gio::Cancellable::NONE,
        )
        .map_err(|e| format!("Dolphin app GetChildren failed: {}", e))?;

    let win_children = win_reply.child_value(0);
    let mut active_win = None;

    for i in 0..win_children.n_children() {
        check_deadline(deadline, "window inspection")?;
        let win = win_children.child_value(i);
        let win_path: String = win.child_get(1);

        let role = get_accessible_role(&a11y_conn, &bus, &win_path, 20);
        let state = get_accessible_state(&a11y_conn, &bus, &win_path, 20);

        if is_active_window(&state, role) {
            active_win = Some(win_path);
            break;
        }
    }

    let win_path = active_win.ok_or_else(|| "no active Dolphin window found".to_string())?;

    // 6. Extract candidate directory from active window title
    let win_name = get_accessible_name(&a11y_conn, &bus, &win_path, 20);
    let win_dir = parse_dir_from_window_title(&win_name);

    // 7. Descend active window tree to find focused/selectable view and selected item
    let mut selected_filename = None;
    let mut location_bar_dir = None;

    walk_window(
        &a11y_conn,
        &bus,
        &win_path,
        0,
        deadline,
        &mut selected_filename,
        &mut location_bar_dir,
    )?;

    let filename = selected_filename.ok_or_else(|| "no item selected in active Dolphin window".to_string())?;

    // 8. Derive absolute path
    let mut candidate_dirs = Vec::new();
    if let Some(dir) = win_dir {
        candidate_dirs.push(dir);
    }
    if let Some(dir) = location_bar_dir {
        if !candidate_dirs.contains(&dir) {
            candidate_dirs.push(dir);
        }
    }

    derive_selected_path(&candidate_dirs, &filename)
}

fn walk_window(
    conn: &gio::DBusConnection,
    bus: &str,
    path: &str,
    depth: usize,
    deadline: Instant,
    selected_filename: &mut Option<String>,
    location_bar_dir: &mut Option<PathBuf>,
) -> Result<(), String> {
    if selected_filename.is_some() || depth > 10 {
        return Ok(());
    }
    check_deadline(deadline, "tree walk")?;

    let role = get_accessible_role(conn, bus, path, 15);

    // Prune unneeded UI trees
    if should_prune_role(role) {
        return Ok(());
    }

    // Inspect combo box for path bar / breadcrumb directory fallback
    if role == ROLE_COMBO_BOX && location_bar_dir.is_none() {
        let name = get_accessible_name(conn, bus, path, 15);
        let trimmed = name.trim();
        let p = Path::new(trimmed);
        if p.is_absolute() && p.is_dir() {
            *location_bar_dir = Some(p.to_path_buf());
        }
    }

    // Inspect view for selection
    if is_view_role(role) {
        // Query NSelectedChildren
        if let Ok(n_sel_reply) = conn.call_sync(
            Some(bus),
            path,
            "org.freedesktop.DBus.Properties",
            "Get",
            Some(&("org.a11y.atspi.Selection", "NSelectedChildren").to_variant()),
            None,
            gio::DBusCallFlags::NO_AUTO_START,
            20,
            gio::Cancellable::NONE,
        ) {
            let n_var: glib::Variant = n_sel_reply.child_get(0);
            let n: i32 = n_var.get().unwrap_or(0);
            if n > 0 {
                // Primary item: index 0
                if let Ok(sel_reply) = conn.call_sync(
                    Some(bus),
                    path,
                    "org.a11y.atspi.Selection",
                    "GetSelectedChild",
                    Some(&(0i32,).to_variant()),
                    None,
                    gio::DBusCallFlags::NO_AUTO_START,
                    20,
                    gio::Cancellable::NONE,
                ) {
                    let sel_obj = sel_reply.child_value(0);
                    let sel_path: String = sel_obj.child_get(1);
                    let item_name = get_accessible_name(conn, bus, &sel_path, 20);
                    if !item_name.is_empty() {
                        *selected_filename = Some(item_name);
                        return Ok(());
                    }
                }
            }
        }
    }

    // Recurse children
    if let Ok(c_reply) = conn.call_sync(
        Some(bus),
        path,
        "org.a11y.atspi.Accessible",
        "GetChildren",
        None,
        None,
        gio::DBusCallFlags::NO_AUTO_START,
        20,
        gio::Cancellable::NONE,
    ) {
        let children_v = c_reply.child_value(0);
        for i in 0..children_v.n_children() {
            if selected_filename.is_some() {
                break;
            }
            let child = children_v.child_value(i);
            let child_path: String = child.child_get(1);
            walk_window(
                conn,
                bus,
                &child_path,
                depth + 1,
                deadline,
                selected_filename,
                location_bar_dir,
            )?;
        }
    }

    Ok(())
}

fn get_accessible_name(conn: &gio::DBusConnection, bus: &str, path: &str, timeout_ms: i32) -> String {
    conn.call_sync(
        Some(bus),
        path,
        "org.freedesktop.DBus.Properties",
        "Get",
        Some(&("org.a11y.atspi.Accessible", "Name").to_variant()),
        None,
        gio::DBusCallFlags::NO_AUTO_START,
        timeout_ms,
        gio::Cancellable::NONE,
    )
    .map(|v| {
        let inner: glib::Variant = v.child_get(0);
        inner.str().unwrap_or("").to_string()
    })
    .unwrap_or_default()
}

fn get_accessible_role(conn: &gio::DBusConnection, bus: &str, path: &str, timeout_ms: i32) -> u32 {
    conn.call_sync(
        Some(bus),
        path,
        "org.a11y.atspi.Accessible",
        "GetRole",
        None,
        None,
        gio::DBusCallFlags::NO_AUTO_START,
        timeout_ms,
        gio::Cancellable::NONE,
    )
    .map(|v| v.child_get::<u32>(0))
    .unwrap_or(0)
}

fn get_accessible_state(conn: &gio::DBusConnection, bus: &str, path: &str, timeout_ms: i32) -> Vec<u32> {
    conn.call_sync(
        Some(bus),
        path,
        "org.a11y.atspi.Accessible",
        "GetState",
        None,
        None,
        gio::DBusCallFlags::NO_AUTO_START,
        timeout_ms,
        gio::Cancellable::NONE,
    )
    .map(|v| {
        let inner = v.child_value(0);
        let mut res = Vec::new();
        for i in 0..inner.n_children() {
            res.push(inner.child_get::<u32>(i));
        }
        res
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_parse_dir_from_window_title_em_dash() {
        let title = "/home/seemoo/Documents/Projects/QuickPeek/tests/fixtures — Dolphin";
        let res = parse_dir_from_window_title(title);
        assert_eq!(
            res,
            Some(PathBuf::from("/home/seemoo/Documents/Projects/QuickPeek/tests/fixtures"))
        );
    }

    #[test]
    fn test_parse_dir_from_window_title_hyphen() {
        let title = "/home/seemoo/Documents/Projects/QuickPeek/tests/fixtures - Dolphin";
        let res = parse_dir_from_window_title(title);
        assert_eq!(
            res,
            Some(PathBuf::from("/home/seemoo/Documents/Projects/QuickPeek/tests/fixtures"))
        );
    }

    #[test]
    fn test_parse_dir_from_window_title_en_dash() {
        let title = "/home/seemoo/Documents/Projects/QuickPeek/tests/fixtures – Dolphin";
        let res = parse_dir_from_window_title(title);
        assert_eq!(
            res,
            Some(PathBuf::from("/home/seemoo/Documents/Projects/QuickPeek/tests/fixtures"))
        );
    }

    #[test]
    fn test_parse_dir_from_window_title_tilde_expansion() {
        if let Ok(home) = std::env::var("HOME") {
            let title = "~/Documents — Dolphin";
            let res = parse_dir_from_window_title(title);
            let expected = PathBuf::from(home).join("Documents");
            if expected.is_dir() {
                assert_eq!(res, Some(expected));
            }
        }
    }

    #[test]
    fn test_parse_dir_from_window_title_non_directory_or_empty() {
        assert_eq!(parse_dir_from_window_title(""), None);
        assert_eq!(parse_dir_from_window_title("   "), None);
        assert_eq!(parse_dir_from_window_title("Dolphin"), None);
        assert_eq!(
            parse_dir_from_window_title("/nonexistent/directory/12345 — Dolphin"),
            None
        );
    }

    #[test]
    fn test_is_active_window() {
        // Frame role = 23, state with bit 1 set (2)
        assert!(is_active_window(&[2, 0], ROLE_FRAME));
        assert!(is_active_window(&[0x43200102, 0], ROLE_FRAME));

        // Frame role but inactive
        assert!(!is_active_window(&[0, 0], ROLE_FRAME));
        assert!(!is_active_window(&[0x43200100, 0], ROLE_FRAME));

        // Active bit set, but not a window role (e.g. list view)
        assert!(!is_active_window(&[2, 0], ROLE_LIST));
        assert!(!is_active_window(&[2, 0], ROLE_PUSH_BUTTON));
    }

    #[test]
    fn test_should_prune_role() {
        assert!(should_prune_role(ROLE_MENU_BAR));
        assert!(should_prune_role(ROLE_TOOL_BAR));
        assert!(should_prune_role(ROLE_PUSH_BUTTON));
        assert!(should_prune_role(ROLE_SCROLL_BAR));

        assert!(!should_prune_role(ROLE_FRAME));
        assert!(!should_prune_role(ROLE_LIST));
        assert!(!should_prune_role(ROLE_TABLE));
        assert!(!should_prune_role(ROLE_COMBO_BOX));
    }

    #[test]
    fn test_is_view_role() {
        assert!(is_view_role(ROLE_LIST));
        assert!(is_view_role(ROLE_TABLE));
        assert!(is_view_role(ROLE_TREE_TABLE));

        assert!(!is_view_role(ROLE_FRAME));
        assert!(!is_view_role(ROLE_COMBO_BOX));
        assert!(!is_view_role(ROLE_PUSH_BUTTON));
    }

    #[test]
    fn test_derive_selected_path_existing_fixture() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = manifest_dir.join("tests").join("fixtures");
        let dirs = vec![fixtures_dir.clone()];

        let res = derive_selected_path(&dirs, "sample.png");
        assert_eq!(res, Ok(fixtures_dir.join("sample.png")));
    }

    #[test]
    fn test_derive_selected_path_empty_filename() {
        let dirs = vec![PathBuf::from("/tmp")];
        assert!(derive_selected_path(&dirs, "").is_err());
        assert!(derive_selected_path(&dirs, "   ").is_err());
    }

    #[test]
    fn test_derive_selected_path_no_candidate_dirs() {
        assert!(derive_selected_path(&[], "sample.png").is_err());
    }

    #[test]
    fn test_derive_selected_path_fallback_to_first_dir() {
        let dirs = vec![PathBuf::from("/nonexistent/dir1"), PathBuf::from("/nonexistent/dir2")];
        let res = derive_selected_path(&dirs, "foo.png");
        assert_eq!(res, Ok(PathBuf::from("/nonexistent/dir1/foo.png")));
    }

    #[test]
    fn test_check_deadline() {
        let future = Instant::now() + Duration::from_secs(10);
        assert!(check_deadline(future, "test").is_ok());

        let past = Instant::now() - Duration::from_millis(10);
        let err = check_deadline(past, "test").unwrap_err();
        assert!(err.contains("deadline exceeded"));
    }

    #[test]
    fn test_primary_item_choice_first_valid() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = manifest_dir.join("tests").join("fixtures");
        let dirs = vec![fixtures_dir.clone()];

        // If a selection has multiple items, primary item choice picks index 0
        let items = vec!["sample.png", "sample2.png"];
        let primary = items[0];
        let res = derive_selected_path(&dirs, primary);
        assert_eq!(res, Ok(fixtures_dir.join("sample.png")));
    }

    #[test]
    fn test_mock_variant_parsing_object_reference() {
        // Mock (so) tuple: (bus_name, object_path)
        let bus = ":1.42";
        let path = "/org/a11y/atspi/accessible/123";
        let v = (bus, path).to_variant();

        let parsed_bus: String = v.child_get(0);
        let parsed_path: String = v.child_get(1);
        assert_eq!(parsed_bus, bus);
        assert_eq!(parsed_path, path);
    }

    #[test]
    fn test_mock_variant_parsing_state_array() {
        // Mock au state array: [0x43200102, 0]
        let states: Vec<u32> = vec![0x43200102, 0];
        let v = states.to_variant();
        let mut parsed = Vec::new();
        for i in 0..v.n_children() {
            parsed.push(v.child_get::<u32>(i));
        }
        assert_eq!(parsed, vec![0x43200102, 0]);
        assert!(is_active_window(&parsed, ROLE_FRAME));
    }
}
