# vim: et:sw=2:ts=2
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
     crane.url = "github:ipetkov/crane";
  };
  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        craneLib = crane.mkLib pkgs;
        packages = pkgs.callPackage ./package.nix {
          inherit craneLib;
        };
        debugPkgs = with pkgs; [
          tcpdump
          natscli
        ];
      in {
        packages = {
          # Expose both binaries
          nats-agent = packages.nats-agent;
          producer = packages.producer;
          default = packages.nats-agent;

          # Container only for the agent
          nats-agent-container = pkgs.dockerTools.streamLayeredImage {
            name = "poc-nats-agent";
            tag = "latest";
            contents = [
              pkgs.busybox
              pkgs.cacert
              pkgs.busybox
              pkgs.bashInteractive
              pkgs.jq
            ] ++ debugPkgs;
            config = {
              EntryPoint = [ "${packages.nats-agent}/bin/nats-agent" ];
              Labels = {
                "org.opencontainers.image.source" = "https://github.com/petee/poc-nats-agent";
                "org.opencontainers.image.description" = "Container image for poc-nats-agent";
              };
            };
          };
        };
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            natscli
            nkeys
            kubernetes-helm
            kubectl
            k9s
            skopeo

            # dev tooling
            tilt
            minikube

            # secrets
            sops
            azure-cli

            python313
            rust-bin.beta.latest.default
            # uv
          ] ++ debugPkgs;
          shellHook = ''
            export NATS_URL=127.0.0.1:4222
            export NATS_STREAM_NAME=events
            export NATS_SUBJECTS="events.>"
            export NATS_CONSUMER_NAME=all-events
            export OTEL_SERVICE_NAME=nats-agent
            export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317
            export PROJECT_DIR="$(git rev-parse --show-toplevel)"

            # We use this key to encrypt any sensitive data (eg: helm values.yaml files)
            export SOPS_AZURE_KEYVAULT_URLS=https://kv-corp-sd8j.vault.azure.net/keys/sops-kv-corp-sd8j/cce212d968b94aeaa5b5752acfd6885a

            echo
            echo "Decrypting secrets using keyvault: $SOPS_AZURE_KEYVAULT_URLS"
            eval "$(sops decrypt $PROJECT_DIR/secrets.yaml --extract '["env"]')"
            echo "Decryption done. Environment variables are set."
          '';
        };
      });
}
