//! One real request of the version check, by hand (plan B6, T4, step 4): `scripts/gui.sh run cargo run -p galleria-gui --example update-probe`.
//! It is the only proof that the TLS of libsoup works in the image. It asks the real address once and says what the app would make of the
//! answer. Not run by any test and not by CI: it touches the network.
#![allow(clippy::print_stdout)]

#[cfg(target_os = "linux")]
fn main() {
    use galleria_gui::{
        update::{Fetch, RELEASES_LATEST, parse},
        update_soup::SoupFetch,
    };
    std::thread::spawn(|| gtk::glib::MainLoop::new(None, false).run());
    std::thread::sleep(std::time::Duration::from_millis(200));
    let started = std::time::Instant::now();
    match SoupFetch::new().latest(RELEASES_LATEST) {
        Ok((status, location)) => {
            println!("status: {status}");
            println!("location: {}", location.as_deref().unwrap_or("(none)"));
            println!(
                "version known: {}",
                parse(status, location.as_deref())
                    .map_or_else(|| "none".to_owned(), |v| v.to_string())
            );
        }
        Err(kind) => println!("no answer: {kind}"),
    }
    println!("took: {} ms", started.elapsed().as_millis());
}

#[cfg(not(target_os = "linux"))]
fn main() {}
