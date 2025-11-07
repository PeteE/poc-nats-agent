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
            name = "ghcr.io/petee/poc-nats-agent";
            contents = [
              pkgs.busybox
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

            # tilt stuff
            tilt

            python313
            rust-bin.beta.latest.default
            # uv
          ] ++ debugPkgs;
          shellHook = ''
            export NATS_URL=10.105.115.71:4222
            export NATS_STREAM_NAME=events
            export NATS_SUBJECTS="events.*"
            export NATS_CONSUMER_NAME=all-events
            export OTEL_SERVICE_NAME=nats-agent
            export OTEL_EXPORTER_OTLP_ENDPOINT=10.97.25.182:4317
          '';
        };
      });
}
