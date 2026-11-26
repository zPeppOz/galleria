//! The tray of the app on Tauri (plan B6, T1): makes the icon and its menu, writes the menu again when the model says so, and
//! turns the click on an entry into what it asks for. Everything that decides is in `tray.rs`.
//!
//! The icon is made inside a barrier: `libappindicator-sys` panics when none of its two libraries is there, and the app has to
//! start without a tray (Q53, point 2 of the review) with one line on `stderr`.

use crate::{
    closing::{Closing, Intent},
    tray::{Factory, Surface, Tray, TrayAction, TrayEntry, TrayIcon as Icon, literal},
    window::{self, WindowControl},
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::Arc,
};
use tauri::{
    AppHandle, Emitter, Manager, Runtime,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
};
use uuid::Uuid;

/// The page is asked to do what the click on the menu says: `{ action: "connect", id }` or `{ action: "disconnect" }` (P6).
pub const EV_TRAY_ACTION: &str = "galleria:tray-action";

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ActionPayload {
    Connect { id: Uuid },
    Disconnect,
}

const ID: &str = "galleria";

fn icon_image(icon: Icon) -> Image<'static> {
    match icon {
        Icon::Off => tauri::include_image!("icons/tray-off.png"),
        Icon::On => tauri::include_image!("icons/tray-on.png"),
        Icon::Busy => tauri::include_image!("icons/tray-busy.png"),
    }
}

struct TauriSurface<R: Runtime> {
    app: AppHandle<R>,
    tray: TrayIcon<R>,
    menu: Menu<R>,
    icon: Option<Icon>,
}

impl<R: Runtime> Surface for TauriSurface<R> {
    fn show(&mut self, entries: &[TrayEntry], icon: Icon, generation: u64) {
        let icon_changed = self.icon != Some(icon);
        self.icon = Some(icon);
        let (app, menu, tray) = (self.app.clone(), self.menu.clone(), self.tray.clone());
        let entries = entries.to_vec();
        // The thread that gets here is the one of the events, and the menu belongs to the thread of the window: the work is posted,
        // never waited for (the window may be waiting for the lock of the tray).
        let posted = self.app.run_on_main_thread(move || {
            if let Err(e) = rewrite(&app, &menu, &entries, generation) {
                eprintln!("galleria-gui: cannot write the menu of the tray: {e}");
            }
            if icon_changed && let Err(e) = tray.set_icon(Some(icon_image(icon))) {
                eprintln!("galleria-gui: cannot change the icon of the tray: {e}");
            }
        });
        if let Err(e) = posted {
            eprintln!("galleria-gui: cannot post the menu of the tray: {e}");
        }
    }
}

/// A menu that is put cannot be taken off or replaced (Tauri, Linux): its content is changed.
fn rewrite<R: Runtime>(
    app: &AppHandle<R>,
    menu: &Menu<R>,
    entries: &[TrayEntry],
    generation: u64,
) -> tauri::Result<()> {
    while menu.remove_at(0)?.is_some() {}
    for (index, entry) in entries.iter().enumerate() {
        match entry {
            TrayEntry::Separator => menu.append(&PredefinedMenuItem::separator(app)?)?,
            TrayEntry::Label { text } => {
                menu.append(&MenuItem::with_id(
                    app,
                    format!("tray:{generation}:{index}"),
                    literal(text),
                    false,
                    None::<&str>,
                )?)?;
            }
            TrayEntry::Item { text, enabled, .. } => {
                menu.append(&MenuItem::with_id(
                    app,
                    format!("tray:{generation}:{index}"),
                    literal(text),
                    *enabled,
                    None::<&str>,
                )?)?;
            }
        }
    }
    Ok(())
}

/// Where the icon of the tray writes its PNG: the folder of the app in the runtime folder (`0700`, Q36), not `/tmp`.
fn icon_dir() -> Option<PathBuf> {
    let dir = crate::instance::runtime_dir(std::env::var_os("XDG_RUNTIME_DIR"))?;
    dir.is_dir().then_some(dir)
}

fn build<R: Runtime>(app: &AppHandle<R>) -> Option<Box<dyn Surface>> {
    let made = (|| -> tauri::Result<TauriSurface<R>> {
        let menu = Menu::new(app)?;
        let mut builder = TrayIconBuilder::with_id(ID)
            .icon(icon_image(Icon::Off))
            .menu(&menu)
            .on_menu_event(|app, event| {
                let id = event.id().as_ref();
                let mut parts = id.strip_prefix("tray:").unwrap_or_default().split(':');
                let (Some(generation), Some(index)) = (
                    parts.next().and_then(|g| g.parse::<u64>().ok()),
                    parts.next().and_then(|i| i.parse::<usize>().ok()),
                ) else {
                    return;
                };
                let action = app
                    .try_state::<Arc<Tray>>()
                    .and_then(|tray| tray.action(generation, index));
                if let Some(action) = action {
                    run_action(app, &action);
                }
            });
        if let Some(dir) = icon_dir() {
            builder = builder.temp_dir_path(dir);
        }
        let tray = builder.build(app)?;
        Ok(TauriSurface {
            app: app.clone(),
            tray,
            menu,
            icon: None,
        })
    })();
    match made {
        Ok(surface) => Some(Box::new(surface)),
        Err(e) => {
            eprintln!("galleria-gui: no tray: {e}");
            None
        }
    }
}

/// The surface of the real app, made once. The library of the indicator that is missing is a panic of its binding: it is
/// stopped here, without the message of the panic hook, and the app goes on without a tray.
pub fn factory<R: Runtime>(app: AppHandle<R>) -> Factory {
    Box::new(move || {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let made = catch_unwind(AssertUnwindSafe(|| build(&app)));
        std::panic::set_hook(previous);
        match made {
            Ok(surface) => surface,
            Err(_) => {
                eprintln!(
                    "galleria-gui: no tray: the library of the indicator (libayatana-appindicator3 or libappindicator3) is not there"
                );
                None
            }
        }
    })
}

/// What a click asks for. «Open» shows the window; «Quit» is the decision of the closing; a connection is asked of the page, which
/// has all the rules of B5 (the replacement, the foreign session, the credentials) and is alive with the window hidden (Q52).
pub fn run_action<R: Runtime>(app: &AppHandle<R>, action: &TrayAction) {
    match action {
        TrayAction::Open => match app.try_state::<Arc<dyn WindowControl>>() {
            Some(window) => window.show(),
            None => window::raise(app),
        },
        TrayAction::Quit => {
            // the same decision as the window: with changes that are not saved the window is shown and the page asks
            if let (Some(closing), Some(window)) = (
                app.try_state::<Arc<Closing>>(),
                app.try_state::<Arc<dyn WindowControl>>(),
            ) {
                closing.request(&**window, Intent::Quit);
            }
        }
        TrayAction::Connect(id) => emit_action(app, &ActionPayload::Connect { id: *id }),
        TrayAction::Disconnect => emit_action(app, &ActionPayload::Disconnect),
    }
}

fn emit_action<R: Runtime>(app: &AppHandle<R>, payload: &ActionPayload) {
    if let Err(e) = app.emit(EV_TRAY_ACTION, payload) {
        eprintln!("galleria-gui: cannot emit {EV_TRAY_ACTION}: {e}");
    }
}

/// The favourites of the rubrica in the order of its list, with their names.
pub fn favourites_of(rubrica: &crate::rubrica::Rubrica) -> Vec<(Uuid, String)> {
    rubrica
        .list()
        .map(|view| {
            view.profiles
                .into_iter()
                .filter(|p| p.favorite)
                .map(|p| (p.id, p.name))
                .collect()
        })
        .unwrap_or_default()
}
