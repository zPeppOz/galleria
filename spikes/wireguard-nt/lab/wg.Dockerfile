# Server WireGuard di prova (laboratorio Galleria, spike 1): stessa rete Docker e stesso "sito" dual-stack dello spike 2
FROM galleria-lab-server
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends wireguard-tools && apt-get clean
COPY wg-entrypoint.sh /wg-entrypoint.sh
RUN chmod +x /wg-entrypoint.sh
CMD ["/wg-entrypoint.sh"]
