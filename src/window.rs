use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

#[derive(Debug, PartialEq, Eq)]
pub enum CliMode {
    Toggle,
    Show(PathBuf),
}

pub fn parse_cli_args_with_dir(args: &[String], current_dir: &Path) -> Result<CliMode, String> {
    if args.len() < 2 {
        Ok(CliMode::Toggle)
    } else if args.len() == 2 {
        let raw_path = Path::new(&args[1]);
        let abs_path = crate::dbus::absolutize_path(raw_path, current_dir);
        Ok(CliMode::Show(abs_path))
    } else {
        Err("too many arguments: at most one image path argument is allowed".to_string())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ToggleAction {
    Close,
    Open,
    NoSelection,
}

pub fn decide_toggle_action(window_open: bool, has_selection: bool) -> ToggleAction {
    if window_open {
        ToggleAction::Close
    } else if has_selection {
        ToggleAction::Open
    } else {
        ToggleAction::NoSelection
    }
}

pub fn calculate_aspect_fit(
    img_w: i32,
    img_h: i32,
    monitor_w: i32,
    monitor_h: i32,
    cap_fraction: f64,
) -> (i32, i32) {
    if img_w <= 0 || img_h <= 0 || monitor_w <= 0 || monitor_h <= 0 {
        return (img_w.max(1), img_h.max(1));
    }

    let max_w = (monitor_w as f64) * cap_fraction;
    let max_h = (monitor_h as f64) * cap_fraction;

    let scale_w = max_w / (img_w as f64);
    let scale_h = max_h / (img_h as f64);

    // Downscaled only if it exceeds aspect-fit cap; NEVER upscale
    let scale = scale_w.min(scale_h).min(1.0);

    let target_w = (img_w as f64 * scale).round() as i32;
    let target_h = (img_h as f64 * scale).round() as i32;

    (target_w, target_h)
}

pub struct WindowManager {
    display: gdk::Display,
    window: Option<gtk4::Window>,
    picture: Option<gtk4::Picture>,
    is_open: Rc<Cell<bool>>,
    current_selection: Option<PathBuf>,
}

impl WindowManager {
    pub fn new() -> Self {
        let display = gdk::Display::default().expect("no display connection");

        let css_provider = gtk4::CssProvider::new();
        css_provider.load_from_data("window.quickpeek-window { background-color: #1E1E1E; }");
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        Self {
            display,
            window: None,
            picture: None,
            is_open: Rc::new(Cell::new(false)),
            current_selection: None,
        }
    }

    #[allow(dead_code)]
    pub fn is_open(&self) -> bool {
        self.is_open.get()
    }

    #[allow(dead_code)]
    pub fn current_selection(&self) -> Option<&PathBuf> {
        self.current_selection.as_ref()
    }

    pub fn show_file(
        &mut self,
        path: &Path,
        start_time: Instant,
    ) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("file not found: {}", path.display()));
        }
        if !path.is_file() {
            return Err(format!("path is not a regular file: {}", path.display()));
        }

        let gio_file = gio::File::for_path(path);
        let texture = gdk::Texture::from_file(&gio_file)
            .map_err(|e| format!("failed to decode image: {}", e))?;

        let img_w = texture.width();
        let img_h = texture.height();

        let (monitor_w, monitor_h) = self
            .display
            .monitors()
            .item(0)
            .and_downcast::<gdk::Monitor>()
            .map(|m| {
                let geom = m.geometry();
                (geom.width(), geom.height())
            })
            .unwrap_or((1920, 1080));

        let (target_w, target_h) =
            calculate_aspect_fit(img_w, img_h, monitor_w, monitor_h, 0.60);

        let show_start = Instant::now();

        if self.is_open.get() {
            if let (Some(win), Some(pic)) = (&self.window, &self.picture) {
                pic.set_paintable(Some(&texture));
                pic.set_size_request(target_w, target_h);
                win.set_default_size(target_w, target_h);
                win.present();

                self.current_selection = Some(path.to_path_buf());
                println!("IMAGE_SWAPPED");
                println!("WARM_MS {}", show_start.elapsed().as_millis());
                return Ok(());
            }
        }

        self.window = None;
        self.picture = None;

        let window = gtk4::Window::new();
        window.add_css_class("quickpeek-window");
        window.set_decorated(false);
        window.set_default_size(target_w, target_h);

        let picture = gtk4::Picture::for_paintable(&texture);
        picture.set_can_shrink(true);
        picture.set_size_request(target_w, target_h);
        window.set_child(Some(&picture));

        let map_start = start_time;
        window.connect_map(move |_| {
            let elapsed = map_start.elapsed().as_millis();
            println!("MAP_MS {}", elapsed);
        });

        let key_controller = gtk4::EventControllerKey::new();
        let win_for_key = window.clone();
        key_controller.connect_key_pressed(move |_ctrl, key, _keycode, _state| {
            if key == gdk::Key::Escape {
                win_for_key.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        window.add_controller(key_controller);

        let is_open_clone = self.is_open.clone();
        self.is_open.set(true);

        window.connect_close_request(move |_| {
            if is_open_clone.get() {
                is_open_clone.set(false);
                println!("WINDOW_CLOSED");
            }
            glib::Propagation::Proceed
        });

        window.present();

        self.current_selection = Some(path.to_path_buf());
        println!("WINDOW_OPENED");
        println!("WARM_MS {}", show_start.elapsed().as_millis());

        self.window = Some(window);
        self.picture = Some(picture);

        Ok(())
    }

    pub fn close_window(&mut self) {
        if let Some(win) = self.window.take() {
            self.picture = None;
            if self.is_open.get() {
                self.is_open.set(false);
                win.close();
                println!("WINDOW_CLOSED");
            }
        }
    }

    pub fn toggle(&mut self) -> Result<(), String> {
        match decide_toggle_action(self.is_open.get(), self.current_selection.is_some()) {
            ToggleAction::Close => {
                let toggle_start = Instant::now();
                self.close_window();
                println!("TOGGLE_CLOSED");
                println!("TOGGLE_MS {}", toggle_start.elapsed().as_millis());
                Ok(())
            }
            ToggleAction::Open => {
                let path = self.current_selection.clone().unwrap();
                let toggle_start = Instant::now();
                println!("TOGGLE_OPENED");
                self.show_file(&path, toggle_start)?;
                println!("TOGGLE_MS {}", toggle_start.elapsed().as_millis());
                Ok(())
            }
            ToggleAction::NoSelection => {
                Err("no file to preview".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_dispatch_no_args() {
        let args = vec!["quickpeek".to_string()];
        let res = parse_cli_args_with_dir(&args, Path::new("/base"));
        assert_eq!(res, Ok(CliMode::Toggle));
    }

    #[test]
    fn test_cli_dispatch_path_relative() {
        let args = vec![
            "quickpeek".to_string(),
            "tests/fixtures/sample.png".to_string(),
        ];
        let res = parse_cli_args_with_dir(&args, Path::new("/base"));
        assert_eq!(
            res,
            Ok(CliMode::Show(PathBuf::from("/base/tests/fixtures/sample.png")))
        );
    }

    #[test]
    fn test_cli_dispatch_path_absolute() {
        let args = vec![
            "quickpeek".to_string(),
            "/var/tmp/sample.png".to_string(),
        ];
        let res = parse_cli_args_with_dir(&args, Path::new("/base"));
        assert_eq!(
            res,
            Ok(CliMode::Show(PathBuf::from("/var/tmp/sample.png")))
        );
    }

    #[test]
    fn test_cli_dispatch_too_many_args() {
        let args = vec![
            "quickpeek".to_string(),
            "sample.png".to_string(),
            "extra.png".to_string(),
        ];
        let res = parse_cli_args_with_dir(&args, Path::new("/base"));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("too many arguments"));
    }

    #[test]
    fn test_toggle_decision_window_open_with_selection() {
        assert_eq!(decide_toggle_action(true, true), ToggleAction::Close);
    }

    #[test]
    fn test_toggle_decision_window_open_no_selection() {
        assert_eq!(decide_toggle_action(true, false), ToggleAction::Close);
    }

    #[test]
    fn test_toggle_decision_window_closed_with_selection() {
        assert_eq!(decide_toggle_action(false, true), ToggleAction::Open);
    }

    #[test]
    fn test_toggle_decision_window_closed_no_selection() {
        assert_eq!(decide_toggle_action(false, false), ToggleAction::NoSelection);
    }

    #[test]
    fn test_aspect_fit_4000x3000_downscaled() {
        // 4000x3000 image on 1920x1080@60% -> 864x648
        let (w, h) = calculate_aspect_fit(4000, 3000, 1920, 1080, 0.60);
        assert_eq!((w, h), (864, 648));
    }

    #[test]
    fn test_aspect_fit_64x64_never_upscale() {
        // 64x64 image on 1920x1080 -> stays 64x64 (never upscale)
        let (w, h) = calculate_aspect_fit(64, 64, 1920, 1080, 0.60);
        assert_eq!((w, h), (64, 64));
    }

    #[test]
    fn test_aspect_fit_500x2000_tall() {
        // 500x2000 image on 1920x1080@60% -> 162x648
        let (w, h) = calculate_aspect_fit(500, 2000, 1920, 1080, 0.60);
        assert_eq!((w, h), (162, 648));
    }
}

