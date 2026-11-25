# Build and test environment for galleria-gui: Ubuntu 24.04 with the Tauri 2 prerequisites for Debian and Ubuntu
# (https://v2.tauri.app/start/prerequisites/), the Node version pinned in .nvmrc and pnpm.
# Built by scripts/gui.sh, which passes NODE_VERSION and PNPM_VERSION from .nvmrc and package.json.
FROM ubuntu:24.04
ARG NODE_VERSION
ARG PNPM_VERSION
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential pkg-config ca-certificates curl wget file git xz-utils xvfb dbus fonts-dejavu-core \
        libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
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
