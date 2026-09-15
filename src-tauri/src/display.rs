use std::{
    env,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

/// Omarchy exports `GDK_BACKEND=wayland,x11,*` for the whole session, so the
/// variable merely being set says nothing about what this app should use. Only
/// a value that names no Wayland backend is somebody's actual choice.
fn prefers_wayland(backend: Option<&str>) -> bool {
    match backend {
        Some(value) => value.contains("wayland") || value.contains('*'),
        None => true,
    }
}

/// WebKitGTK delivers pointer events at the wrong coordinates under fractional
/// Wayland scaling: a click lands hundreds of CSS pixels from where it was
/// aimed, which leaves most of the window unusable. XWayland scales by whole
/// numbers and does not have the fault, so it is what omafil asks for until the
/// GTK side is fixed. Set `OMAFIL_BACKEND=wayland` to overrule that.
pub(crate) fn prefer_xwayland() {
    if env::var_os("WAYLAND_DISPLAY").is_none() {
        return;
    }
    if env::var("OMAFIL_BACKEND").is_ok_and(|backend| backend == "wayland") {
        return;
    }

    let backend = env::var("GDK_BACKEND").ok();
    if prefers_wayland(backend.as_deref()) {
        env::set_var("GDK_BACKEND", "x11");
    }
}

fn socket_path() -> Option<PathBuf> {
    let runtime = env::var_os("XDG_RUNTIME_DIR")?;
    let signature = env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;

    Some(PathBuf::from(runtime).join("hypr").join(signature).join(".socket.sock"))
}

fn request(command: &str) -> Option<String> {
    let mut socket = UnixStream::connect(socket_path()?).ok()?;
    socket.write_all(command.as_bytes()).ok()?;

    let mut reply = String::new();
    socket.read_to_string(&mut reply).ok()?;

    Some(reply)
}

/// The scale of the focused monitor, read out of the compositor's own reply.
/// A full JSON parser is not needed for two flat fields.
pub(crate) fn parse_focused_scale(monitors: &str) -> Option<f64> {
    let mut scale = None;

    for line in monitors.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("\"scale\":") {
            scale = value.trim().trim_end_matches(',').parse::<f64>().ok();
        }
        if line.starts_with("\"focused\":") && line.contains("true") {
            return scale;
        }
    }

    None
}

/// Under XWayland every layer reports a scale of 1, so the compositor is the
/// only thing that still knows the display is fractionally scaled.
pub(crate) fn compositor_scale() -> Option<f64> {
    parse_focused_scale(&request("j/monitors")?).filter(|scale| *scale > 0.0)
}

#[cfg(test)]
mod tests {
    use super::{parse_focused_scale, prefers_wayland};

    #[test]
    fn a_session_default_is_not_a_choice_of_wayland() {
        // What Omarchy exports for every app in the session.
        assert!(prefers_wayland(Some("wayland,x11,*")));
        assert!(prefers_wayland(Some("wayland")));
        assert!(prefers_wayland(None));
    }

    #[test]
    fn naming_no_wayland_backend_is_left_alone() {
        assert!(!prefers_wayland(Some("x11")));
        assert!(!prefers_wayland(Some("broadway")));
    }

    #[test]
    fn reads_the_focused_monitors_scale() {
        let reply = r#"[
 {
  "name": "eDP-1",
  "scale": 1.60,
  "focused": false
 },
 {
  "name": "HDMI-A-1",
  "scale": 2.00,
  "focused": true
 }
]"#;

        assert_eq!(parse_focused_scale(reply), Some(2.0));
    }

    #[test]
    fn a_reply_with_no_focused_monitor_yields_nothing() {
        assert_eq!(parse_focused_scale(r#"[{"scale": 1.60,"focused": false}]"#), None);
    }
}
