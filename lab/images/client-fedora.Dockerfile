FROM fedora:latest
ENV container=docker
RUN dnf install -y --setopt=install_weak_deps=False systemd systemd-resolved dbus-broker \
    openvpn iproute iputils tcpdump curl python3 oathtool procps-ng util-linux shadow-utils \
    && dnf clean all && groupadd galleria && useradd -m -G galleria alice && useradd -m mallory \
    && systemctl mask systemd-binfmt.service proc-sys-fs-binfmt_misc.automount systemd-networkd.service systemd-networkd-wait-online.service \
    && systemctl enable systemd-resolved
RUN dnf install -y --setopt=install_weak_deps=False util-linux-script && dnf clean all
STOPSIGNAL SIGRTMIN+3
CMD ["/sbin/init"]
