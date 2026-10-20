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
      system = pkgs.stdenv.hostPlatform.system;
      craneLib = inputs.crane.mkLib pkgs;
      tometPkgs = inputs.tomet.packages.${system};
      twritPkgs = inputs.twrit.packages.${system};
      shelfPkgs = inputs.tomet-shelf.packages.${system};
      deskPkgs = inputs.sisho-desk.packages.${system};

      # Several packages as one: the `bin/` of each in a single directory.
      combine =
        paths:
        pkgs.symlinkJoin {
          name = "tomet-toolchain";
          inherit paths;
        };
    in
    {
      packages = rec {
        default = sisho;
        sisho = pkgs.callPackage ./pkgs/sisho.nix { inherit craneLib; };

        # The tools, as their own flakes build them.
        inherit (tometPkgs) tomet tomet-lsp;
        inherit (twritPkgs) twrit;
        inherit (shelfPkgs) tomet-shelf;
        inherit (deskPkgs) sisho-desk;

        # The core: what `sisho check` and `sisho fix` run. Add the rest with `combine`.
        toolchain = combine [
          sisho
          tomet
          twrit
          sisho-desk
        ];
      };

      # A function, so it cannot be a package: `legacyPackages.<system>.combine [ ... ]`.
      legacyPackages.combine = combine;

      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs;
        fenix = inputs.fenix.packages.${system};
        inherit (tometPkgs) tomet tomet-lsp;
        inherit (twritPkgs) twrit;
        inherit (deskPkgs) sisho-desk;
      };

      treefmt = import ./formatter.nix {
        inherit (tometPkgs) tomet;
      };
    };
}
