# vim: et:sw=2:ts=2
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # nats.io stuff
            natscli
            nkeys

            # tilt stuff
            tilt

            python313
            # pkg-config
            rust-bin.beta.latest.default
            # rustup
            # cargo
            uv
          ];
          shellHook = ''
          '';
        };
      });
}
