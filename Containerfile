FROM gcr.io/k8s-minikube/kicbase:v0.0.50
# fix nftables discovery
RUN sed -i 's|if \[ "${num_legacy_lines}" -ge "${num_nft_lines}" \]; then|if iptables-legacy -L >/dev/null 2>\&1 \&\& [ "${num_legacy_lines}" -ge "${num_nft_lines}" ]; then|' /usr/local/bin/entrypoint \
 && grep -n 'num_legacy_lines.*-ge' /usr/local/bin/entrypoint
