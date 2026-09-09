{
  craneLib,
  lib,
  ...
}:
let
  commonArgs = {
    src = craneLib.cleanCargoSource ./.;
    strictDeps = true;
  };

  # avoid updating dependencies on unrelated changes
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  # Build all binaries together (for shared dependencies)
  allBinaries = craneLib.buildPackage (
    commonArgs
    // {
      inherit cargoArtifacts;
      passthru = {
        clippy = craneLib.cargoClippy (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets --all-features -- --deny warnings --no-deps";
          }
        );
      };
    }
  );
in
{
  # Main agent binary
  nats-agent = allBinaries.overrideAttrs (old: {
    postInstall = ''
      mkdir -p $out/bin
      cp ${allBinaries}/bin/nats-agent $out/bin/
    '';
  });

  # Producer binary
  producer = allBinaries.overrideAttrs (old: {
    pname = "nats-producer";
    postInstall = ''
      mkdir -p $out/bin
      cp ${allBinaries}/bin/producer $out/bin/
    '';
  });
}
