# Client di prova Fedora corrente (OpenVPN 2.7.x dai repository)
FROM fedora:latest
RUN dnf install -y --setopt=install_weak_deps=False \
      openvpn systemd systemd-resolved iproute iputils socat tcpdump curl bind-utils procps-ng \
      python3 util-linux shadow-utils dbus-broker \
    && dnf clean all \
    && useradd -m -s /bin/bash alice \
    && (systemctl mask systemd-networkd.service systemd-networkd.socket systemd-networkd-wait-online.service || true) \
    && systemctl enable systemd-resolved
STOPSIGNAL SIGRTMIN+3
CMD ["/sbin/init"]
