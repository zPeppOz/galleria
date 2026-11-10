# Cross-compilazione dei binari Windows degli spike (target x86_64-pc-windows-gnu) senza toccare l'host
FROM rust:1-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends mingw-w64 && apt-get clean
RUN rustup target add x86_64-pc-windows-gnu
