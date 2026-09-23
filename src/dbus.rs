use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::path::{Path, PathBuf};

pub const BUS_NAME: &str = "org.quickpeek.QuickPeek";
pub const OBJECT_PATH: &str = "/org/quickpeek/QuickPeek";
pub const INTERFACE_NAME: &str = "org.quickpeek.QuickPeek";

pub const INTROSPECTION_XML: &str = r#"<node>
  <interface name="org.quickpeek.QuickPeek">
    <method name="ShowFile">
      <arg name="path" type="s" direction="in"/>
      <arg name="ok" type="b" direction="out"/>
      <arg name="message" type="s" direction="out"/>
    </method>
    <method name="Quit"/>
  </interface>
</node>"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Daemon,
    Client,
}

pub fn map_request_name_reply(reply: u32) -> Result<Role, String> {
    match reply {
        1 => Ok(Role::Daemon),
        3 => Ok(Role::Client),
        other => Err(format!("Unexpected RequestName reply: {}", other)),
    }
}

pub fn absolutize_path(path: &Path, current_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
    }
}

pub fn request_name(conn: &gio::DBusConnection) -> Result<Role, glib::Error> {
    // flags DO_NOT_QUEUE = 0x4
    let params = (BUS_NAME, 0x4u32).to_variant();
    let reply = conn.call_sync(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "RequestName",
        Some(&params),
        None,
        gio::DBusCallFlags::NONE,
        2000,
        gio::Cancellable::NONE,
    )?;
    let code: u32 = reply.child_get(0);
    map_request_name_reply(code).map_err(|e| glib::Error::new(gio::IOErrorEnum::Failed, &e))
}

pub fn call_show_file(
    conn: &gio::DBusConnection,
    path: &Path,
    timeout_ms: i32,
) -> Result<(bool, String), glib::Error> {
    let path_str = path.to_string_lossy();
    let params = (path_str.as_ref(),).to_variant();
    let reply = conn.call_sync(
        Some(BUS_NAME),
        OBJECT_PATH,
        INTERFACE_NAME,
        "ShowFile",
        Some(&params),
        None,
        gio::DBusCallFlags::NONE,
        timeout_ms,
        gio::Cancellable::NONE,
    )?;
    let ok: bool = reply.child_get(0);
    let msg: String = reply.child_get(1);
    Ok((ok, msg))
}

pub fn register_server<F>(
    conn: &gio::DBusConnection,
    handler: F,
) -> Result<gio::RegistrationId, glib::Error>
where
    F: Fn(&str, glib::Variant, gio::DBusMethodInvocation) + 'static,
{
    let node_info = gio::DBusNodeInfo::for_xml(INTROSPECTION_XML)?;
    let iface_info = node_info
        .lookup_interface(INTERFACE_NAME)
        .ok_or_else(|| glib::Error::new(gio::IOErrorEnum::NotFound, "interface not found in XML"))?;

    conn.register_object(OBJECT_PATH, &iface_info)
        .method_call(move |_conn, _sender, _obj_path, _iface, method, params, invocation| {
            handler(method, params, invocation);
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_request_name_reply_daemon() {
        assert_eq!(map_request_name_reply(1), Ok(Role::Daemon));
    }

    #[test]
    fn test_map_request_name_reply_client() {
        assert_eq!(map_request_name_reply(3), Ok(Role::Client));
    }

    #[test]
    fn test_map_request_name_reply_other() {
        assert!(map_request_name_reply(2).is_err());
        assert!(map_request_name_reply(4).is_err());
        assert!(map_request_name_reply(0).is_err());
    }

    #[test]
    fn test_absolutize_path_relative() {
        let base = Path::new("/workspace/project");
        let rel = Path::new("tests/fixtures/sample.png");
        assert_eq!(
            absolutize_path(rel, base),
            PathBuf::from("/workspace/project/tests/fixtures/sample.png")
        );
    }

    #[test]
    fn test_absolutize_path_already_absolute() {
        let base = Path::new("/workspace/project");
        let abs = Path::new("/var/tmp/sample.png");
        assert_eq!(
            absolutize_path(abs, base),
            PathBuf::from("/var/tmp/sample.png")
        );
    }
}
