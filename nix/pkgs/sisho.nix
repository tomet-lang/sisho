{
  craneLib,
}:
let
  src = ../..;

  commonArgs = {
    inherit src;

    pname = "sisho";
    version = "0.1.0";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;

    cargoExtraArgs = "-p sisho-cli";

    doCheck = true;
  }
)
