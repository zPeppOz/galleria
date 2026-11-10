FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      openvpn openssl iproute2 iptables dnsmasq oathtool tcpdump python3 procps iputils-ping \
    && rm -rf /var/lib/apt/lists/*
COPY server/ /etc/openvpn/lab/
RUN chmod +x /etc/openvpn/lab/*.sh
CMD ["/etc/openvpn/lab/entrypoint.sh"]
