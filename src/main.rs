use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::env;
use std::path::PathBuf;
use std::time::Instant;

pub fn validate_cli_path(args: &[String]) -> Result<PathBuf, String> {
    if args.len() < 2 {
        return Err("missing image path argument".to_string());
    }
    if args.len() > 2 {
        return Err("too many arguments: exactly one image path argument is required".to_string());
    }

    let path_str = &args[1];
    let path = PathBuf::from(path_str);

    if !path.exists() {
        return Err(format!("file not found: {}", path_str));
    }

    if !path.is_file() {
        return Err(format!("path is not a file: {}", path_str));
    }

    // Phase 1 formats: PNG and JPEG only
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" => Ok(path),
        _ => Err(format!(
            "unsupported format '{}': Phase 1 supports PNG and JPEG only",
            ext
        )),
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

fn main() {
    let start_time = Instant::now();

    let args: Vec<String> = env::args().collect();
    let image_path = match validate_cli_path(&args) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };

    if let Err(err) = gtk4::init() {
        eprintln!("Error: failed to initialize GTK4: {}", err);
        std::process::exit(1);
    }

    // Load image texture
    let gio_file = gio::File::for_path(&image_path);
    let texture = match gdk::Texture::from_file(&gio_file) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("Error: failed to load image '{}': {}", image_path.display(), err);
            std::process::exit(1);
        }
    };

    let img_w = texture.width();
    let img_h = texture.height();

    // Query monitor geometry
    let display = gdk::Display::default().expect("no display connection");
    let (monitor_w, monitor_h) = display
        .monitors()
        .item(0)
        .and_downcast::<gdk::Monitor>()
        .map(|m| {
            let geom = m.geometry();
            (geom.width(), geom.height())
        })
        .unwrap_or((1920, 1080));

    // Calculate aspect fit size (60% cap, never upscale)
    let (target_w, target_h) = calculate_aspect_fit(img_w, img_h, monitor_w, monitor_h, 0.60);

    // Solid background #1E1E1E (no alpha)
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data("window.quickpeek-window { background-color: #1E1E1E; }");
    gtk4::style_context_add_provider_for_display(
        &display,
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Create undecorated window
    let window = gtk4::Window::new();
    window.add_css_class("quickpeek-window");
    window.set_decorated(false);
    window.set_default_size(target_w, target_h);

    // Picture widget
    let picture = gtk4::Picture::for_paintable(&texture);
    picture.set_can_shrink(true);
    picture.set_size_request(target_w, target_h);
    window.set_child(Some(&picture));

    // Instrumentation: print MAP_MS on first map
    let map_start = start_time;
    window.connect_map(move |_| {
        let elapsed = map_start.elapsed().as_millis();
        println!("MAP_MS {}", elapsed);
    });

    // Main loop setup
    let main_loop = glib::MainLoop::new(None, false);

    // Close on Escape key
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

    // Exit 0 on window close (Escape or WM close)
    let loop_for_close = main_loop.clone();
    window.connect_close_request(move |_| {
        loop_for_close.quit();
        glib::Propagation::Proceed
    });

    window.present();
    main_loop.run();

    std::process::exit(0);
}
