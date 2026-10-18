{
  flake-parts,
  ...
}@inputs:
flake-parts.lib.mkFlake { inherit inputs; } {
  systems = [
    "x86_64-linux"
    "aarch64-linux"
    "aarch64-darwin"
  ];
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem =
    { pkgs, ... }:
    let
      craneLib = inputs.crane.mkLib pkgs;
      tometPkgs = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system};
    in
    {
      packages = rec {
        default = sisho;
        sisho = pkgs.callPackage ./pkgs/sisho.nix { inherit craneLib; };
      };

      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs;
        fenix = inputs.fenix.packages.${pkgs.stdenv.hostPlatform.system};
        tomet = tometPkgs.tomet;
        tomet-lsp = tometPkgs.tomet-lsp;
      };

      treefmt = import ./formatter.nix {
        tomet = tometPkgs.tomet;
      };
    };
}
