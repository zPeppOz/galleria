//! Is there a tray to come back from? (plan B6, T3, S51, Q53): the session bus says it. The tray is shown when `org.kde.StatusNotifierWatcher`
//! has an owner and says `IsStatusNotifierHostRegistered`: on GNOME that is the extension of the indicators, and without it the answer is no. Without a
//! bus, without an owner, or without an answer in 300 ms it is no: a window that cannot come back must not be hidden.

/// How long the bus has to answer, each time it is asked.
pub const ANSWER_MS: i32 = 300;

/// The tray is shown now, on the session bus of this process.
#[must_use]
pub fn tray_is_shown() -> bool {
    #[cfg(target_os = "linux")]
    {
        tray_is_shown_on(std::env::var("DBUS_SESSION_BUS_ADDRESS").ok().as_deref())
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// [`tray_is_shown`] on the bus at `address` (`None`: no bus). A connection of its own for each question: the one of the process is shared
/// and kept, and an answer must not depend on what another call left in it.
#[cfg(target_os = "linux")]
#[must_use]
pub fn tray_is_shown_on(address: Option<&str>) -> bool {
    use gtk::gio::{self, prelude::*};
    let Some(address) = address.filter(|a| !a.is_empty()) else {
        return false;
    };
    let Ok(bus) = gio::DBusConnection::for_address_sync(
        address,
        gio::DBusConnectionFlags::AUTHENTICATION_CLIENT
            | gio::DBusConnectionFlags::MESSAGE_BUS_CONNECTION,
        None,
        gio::Cancellable::NONE,
    ) else {
        return false;
    };
    let call =
        |name: &str, path: &str, interface: &str, method: &str, args: &gtk::glib::Variant| {
            bus.call_sync(
                Some(name),
                path,
                interface,
                method,
                Some(args),
                None,
                gio::DBusCallFlags::NONE,
                ANSWER_MS,
                gio::Cancellable::NONE,
            )
            .ok()
        };
    let owned = call(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameHasOwner",
        &("org.kde.StatusNotifierWatcher",).to_variant(),
    )
    .and_then(|reply| reply.child_value(0).get::<bool>())
    .unwrap_or(false);
    let registered = owned
        && call(
            "org.kde.StatusNotifierWatcher",
            "/StatusNotifierWatcher",
            "org.freedesktop.DBus.Properties",
            "Get",
            &(
                "org.kde.StatusNotifierWatcher",
                "IsStatusNotifierHostRegistered",
            )
                .to_variant(),
        )
        .and_then(|reply| reply.child_value(0).as_variant())
        .and_then(|value| value.get::<bool>())
        .unwrap_or(false);
    let _ = bus.close_sync(gio::Cancellable::NONE);
    registered
}
