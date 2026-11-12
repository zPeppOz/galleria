FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    openvpn openssl iproute2 iptables dnsmasq oathtool python3 iputils-ping wireguard-tools \
    && rm -rf /var/lib/apt/lists/*
COPY server /lab/server
COPY wg /lab/wg
RUN chmod +x /lab/server/*.sh /lab/wg/*.sh
CMD ["/lab/server/entrypoint.sh"]
