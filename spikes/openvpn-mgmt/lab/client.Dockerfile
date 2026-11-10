# Client di prova: ARG BASE = ubuntu:24.04 (OpenVPN 2.6) o ubuntu:26.04 (OpenVPN 2.7)
ARG BASE=ubuntu:24.04
FROM ${BASE}
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      openvpn systemd systemd-sysv systemd-resolved iproute2 iputils-ping socat tcpdump curl \
      dnsutils procps ca-certificates oathtool python3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -s /bin/bash alice \
    && (systemctl mask systemd-networkd.service systemd-networkd.socket systemd-networkd-wait-online.service || true)
STOPSIGNAL SIGRTMIN+3
CMD ["/sbin/init"]
