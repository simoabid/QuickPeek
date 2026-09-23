mod dbus;
mod window;

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

struct DaemonContext {
    wm: window::WindowManager,
    main_loop: glib::MainLoop,
}

fn main() {
    let start_time = Instant::now();

    let args: Vec<String> = env::args().collect();
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cli_mode = match window::parse_cli_args_with_dir(&args, &current_dir) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };

    // Connect to session D-Bus
    let conn = match gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Error: failed to connect to D-Bus session: {}", err);
            std::process::exit(1);
        }
    };

    // Race-free ordering: register the D-Bus object BEFORE acquiring the name
    let daemon_ctx: Rc<RefCell<Option<DaemonContext>>> = Rc::new(RefCell::new(None));
    let ctx_clone = daemon_ctx.clone();

    let _reg_id = match dbus::register_server(&conn, move |method, params, invocation| {
        match method {
            "ShowFile" => {
                let path_str = params.child_get::<String>(0);
                let path = PathBuf::from(path_str);
                let show_start = Instant::now();

                let mut ctx_borrow = ctx_clone.borrow_mut();
                if let Some(ctx) = ctx_borrow.as_mut() {
                    match ctx.wm.show_file(&path, show_start) {
                        Ok(()) => {
                            invocation.return_value(Some(&(true, "").to_variant()));
                        }
                        Err(err) => {
                            invocation.return_value(Some(&(false, err.as_str()).to_variant()));
                        }
                    }
                } else {
                    invocation.return_value(Some(&(false, "daemon not ready").to_variant()));
                }
            }
            "Toggle" => {
                let mut ctx_borrow = ctx_clone.borrow_mut();
                if let Some(ctx) = ctx_borrow.as_mut() {
                    match ctx.wm.toggle() {
                        Ok(()) => {
                            invocation.return_value(Some(&(true, "").to_variant()));
                        }
                        Err(err) => {
                            invocation.return_value(Some(&(false, err.as_str()).to_variant()));
                        }
                    }
                } else {
                    invocation.return_value(Some(&(false, "daemon not ready").to_variant()));
                }
            }
            "Quit" => {
                let mut ctx_borrow = ctx_clone.borrow_mut();
                if let Some(ctx) = ctx_borrow.as_mut() {
                    ctx.wm.close_window();
                    println!("DAEMON_QUIT");
                    let loop_clone = ctx.main_loop.clone();
                    invocation.return_value(None);
                    glib::idle_add_local_once(move || {
                        loop_clone.quit();
                    });
                } else {
                    invocation.return_value(None);
                }
            }
            _ => {
                invocation.return_dbus_error(
                    "org.freedesktop.DBus.Error.UnknownMethod",
                    "Unknown method",
                );
            }
        }
    }) {
        Ok(id) => id,
        Err(err) => {
            eprintln!("Error: failed to register D-Bus object: {}", err);
            std::process::exit(1);
        }
    };

    // Acquire bus name
    let role = match dbus::request_name(&conn) {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Error: D-Bus RequestName failed: {}", err);
            std::process::exit(1);
        }
    };

    match role {
        dbus::Role::Client => {
            match cli_mode {
                window::CliMode::Toggle => {
                    match dbus::call_toggle(&conn, 2000) {
                        Ok((true, _)) => {
                            // On success print NOTHING
                            std::process::exit(0);
                        }
                        Ok((false, msg)) => {
                            eprintln!("Error: {}", msg);
                            std::process::exit(1);
                        }
                        Err(err) => {
                            eprintln!("Error: D-Bus call failed: {}", err);
                            std::process::exit(1);
                        }
                    }
                }
                window::CliMode::Show(abs_path) => {
                    println!("ROLE CLIENT");
                    match dbus::call_show_file(&conn, &abs_path, 2000) {
                        Ok((true, _)) => {
                            println!("CALL_MS {}", start_time.elapsed().as_millis());
                            std::process::exit(0);
                        }
                        Ok((false, msg)) => {
                            eprintln!("Error: {}", msg);
                            std::process::exit(1);
                        }
                        Err(err) => {
                            eprintln!("Error: D-Bus call failed: {}", err);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        dbus::Role::Daemon => {
            println!("ROLE DAEMON");

            if let Err(err) = gtk4::init() {
                eprintln!("Error: failed to initialize GTK4: {}", err);
                std::process::exit(1);
            }

            let wm = window::WindowManager::new();
            let main_loop = glib::MainLoop::new(None, false);

            let context = DaemonContext {
                wm,
                main_loop: main_loop.clone(),
            };
            *daemon_ctx.borrow_mut() = Some(context);

            if let window::CliMode::Show(ref path) = cli_mode {
                let initial_show = daemon_ctx
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .wm
                    .show_file(path, start_time);

                if let Err(err) = initial_show {
                    eprintln!("Error: {}", err);
                }
            }

            main_loop.run();
            std::process::exit(0);
        }
    }
}
