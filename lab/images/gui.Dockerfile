# Build and test environment for galleria-gui: Ubuntu 24.04 with the Tauri 2 prerequisites for Debian and Ubuntu
# (https://v2.tauri.app/start/prerequisites/), what the WebDriver smoke test needs (webkit2gtk-driver, Xvfb, xclip), the
# Node version pinned in .nvmrc and pnpm. tauri-driver is a Rust tool: scripts/gui.sh installs it under target/gui-tools.
# Built by scripts/gui.sh, which passes NODE_VERSION and PNPM_VERSION from .nvmrc and package.json.
FROM ubuntu:24.04
ARG NODE_VERSION
ARG PNPM_VERSION
ENV DEBIAN_FRONTEND=noninteractive
# The package names are those of Ubuntu 24.04 (the base is pinned by tag, not by digest): `webkit2gtk-driver` is called
# `webkitgtk-webdriver` in 26.04, so a newer base needs this line changed.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential pkg-config ca-certificates curl wget file git xz-utils xvfb xauth xclip dbus fonts-dejavu-core webkit2gtk-driver \
        libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*
# The screen reader check (`scripts/gui.sh a11y`, local only: Q22): Orca, the AT-SPI bus and the Python binding that
# lab/tests/atspi-dump.py uses to print the accessibility tree of the window, plus speech-dispatcher, which Orca wants
# to start its speech output. `libglib2.0-bin` gives `gsettings`, and `dconf-gsettings-backend` stores what it sets.
RUN apt-get update && apt-get install -y --no-install-recommends \
        orca at-spi2-core speech-dispatcher python3-gi gir1.2-atspi-2.0 libglib2.0-bin dconf-gsettings-backend dconf-service \
    && rm -rf /var/lib/apt/lists/*
RUN set -eux; \
    test -n "$NODE_VERSION" -a -n "$PNPM_VERSION"; \
    case "$(dpkg --print-architecture)" in amd64) node_arch=x64 ;; arm64) node_arch=arm64 ;; *) exit 1 ;; esac; \
    tarball="node-v${NODE_VERSION}-linux-${node_arch}.tar.xz"; \
    cd /tmp; \
    curl -fsSLO "https://nodejs.org/dist/v${NODE_VERSION}/${tarball}"; \
    curl -fsSLO "https://nodejs.org/dist/v${NODE_VERSION}/SHASUMS256.txt"; \
    grep " ${tarball}\$" SHASUMS256.txt | sha256sum -c -; \
    mkdir /opt/node; \
    tar -xJf "${tarball}" -C /opt/node --strip-components=1; \
    rm -f "${tarball}" SHASUMS256.txt
ENV PATH=/opt/node/bin:$PATH
RUN npm install --global "pnpm@${PNPM_VERSION}" && pnpm --version
