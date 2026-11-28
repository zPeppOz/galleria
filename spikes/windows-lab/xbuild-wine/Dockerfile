# The tests of the crates that need no system, built for Windows (x86_64-pc-windows-gnu) and run under wine64 in a container
# (phase 3, P1-B, Q78). Wine is not Windows: it says that the binaries start and that the portable tests pass, nothing
# about the real system. Not FROM galleria-xbuild: bookworm has wine 8, which lacks bcryptprimitives.dll that the
# standard library of Rust needs; trixie has wine 10.
FROM rust:1-trixie
RUN apt-get update && apt-get install -y --no-install-recommends mingw-w64 wine64 wine libwine && apt-get clean
RUN rustup target add x86_64-pc-windows-gnu
