ARG BASE=ubuntu:24.04
FROM ${BASE}
ENV DEBIAN_FRONTEND=noninteractive container=docker
RUN apt-get update && apt-get install -y --no-install-recommends \
    systemd systemd-sysv systemd-resolved dbus openvpn iproute2 iputils-ping tcpdump curl \
    python3 oathtool procps ca-certificates util-linux passwd \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd galleria && useradd -m -G galleria alice && useradd -m mallory \
    && systemctl mask systemd-binfmt.service proc-sys-fs-binfmt_misc.automount systemd-networkd.service systemd-networkd-wait-online.service \
    && systemctl enable systemd-resolved
STOPSIGNAL SIGRTMIN+3
CMD ["/sbin/init"]
