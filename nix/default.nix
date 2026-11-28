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
      boardPkgs = inputs.sisho-board.packages.${system};
      deskPkgs = inputs.sisho-desk.packages.${system};
      auditPkgs = inputs.sisho-audit.packages.${system};
      historyPkgs = inputs.sisho-history.packages.${system};
      catastoPkgs = inputs.catasto.packages.${system};
      tetchPkgs = inputs.tetch.packages.${system};

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

        # What CI builds and sends to Cachix (.github/workflows/nix-ci.yml). Including `full`
        # ensures every constituent tool across all bundles is cached for `sisho`'s pinned revisions.
        ci = pkgs.linkFarm "ci" [
          {
            name = "sisho";
            path = sisho;
          }
          {
            name = "full";
            path = full;
          }
        ];

        # The tools, as their own flakes build them.
        inherit (tometPkgs) tomet tomet-lsp;
        inherit (twritPkgs) twrit;
        inherit (shelfPkgs) tomet-shelf;
        inherit (catastoPkgs) catasto;
        inherit (tetchPkgs) tetch;
        inherit (boardPkgs) sisho-board;
        inherit (deskPkgs) sisho-desk;
        inherit (auditPkgs) sisho-audit;
        inherit (historyPkgs) sisho-history;

        # Minimal starter bundle for newcomers and PKM beginners.
        starter = combine [
          tomet
          tomet-lsp
          sisho
          sisho-board
          sisho-desk
        ];

        # Ecosystem toolchain for Tomet repositories (core CLI, LSP, shelf, and audit).
        toolchain = combine [
          tomet
          tomet-lsp
          tomet-shelf
          twrit
          catasto
          tetch
          sisho
          sisho-audit
        ];

        # Full ecosystem bundle (`starter` + `toolchain`).
        full = combine [
          tomet
          tomet-lsp
          twrit
          catasto
          tetch
          tomet-shelf
          sisho
          sisho-board
          sisho-desk
          sisho-audit
          sisho-history
        ];
      };

      # A function, so it cannot be a package: `legacyPackages.<system>.combine [ ... ]`.
      legacyPackages.combine = combine;

      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs;
        fenix = inputs.fenix.packages.${system};
        inherit (tometPkgs) tomet tomet-lsp;
        inherit (twritPkgs) twrit;
        inherit (catastoPkgs) catasto;
        inherit (tetchPkgs) tetch;
        inherit (boardPkgs) sisho-board;
        inherit (deskPkgs) sisho-desk;
        inherit (auditPkgs) sisho-audit;
        inherit (historyPkgs) sisho-history;
      };

      treefmt = import ./formatter.nix {
        inherit (tometPkgs) tomet;
      };
    };
}
