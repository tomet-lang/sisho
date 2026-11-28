{
  description = "Sisho - Orchestrator CLI for the Tomet ecosystem";

  nixConfig = {
    extra-substituters = [ "https://tomet.cachix.org" ];
    extra-trusted-public-keys = [ "tomet.cachix.org-1:9c/iO8Tb6YOM+3r55t12W3fOJK+66itPEaIe8Rs9MDw=" ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # --- Rust ---
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
    };

    # --- Tools ---
    tomet = {
      url = "github:tomet-lang/tomet";
      inputs.twrit.follows = "";
      inputs.tomet-shelf.follows = "";
      inputs.catasto.follows = "";
      inputs.sisho.follows = "";
    };
    twrit = {
      url = "github:tomet-lang/twrit";
      inputs.tomet.follows = "";
      inputs.tomet-shelf.follows = "";
      inputs.catasto.follows = "";
      inputs.sisho.follows = "";
    };
    tomet-shelf = {
      url = "github:tomet-lang/tomet-shelf";
      inputs.tomet.follows = "";
      inputs.twrit.follows = "";
      inputs.catasto.follows = "";
      inputs.sisho.follows = "";
    };
    sisho-board = {
      url = "github:tomet-lang/sisho-board";
      inputs.tomet.follows = "";
      inputs.twrit.follows = "";
      inputs.catasto.follows = "";
      inputs.sisho.follows = "";
    };
    sisho-desk = {
      url = "github:tomet-lang/sisho-desk";
      inputs.tomet.follows = "";
      inputs.twrit.follows = "";
      inputs.catasto.follows = "";
      inputs.sisho.follows = "";
    };
    sisho-audit = {
      url = "github:tomet-lang/sisho-audit";
      inputs.tomet.follows = "";
      inputs.twrit.follows = "";
      inputs.catasto.follows = "";
      inputs.sisho.follows = "";
    };
    sisho-history = {
      url = "github:tomet-lang/sisho-history";
      inputs.tomet.follows = "";
      inputs.twrit.follows = "";
      inputs.catasto.follows = "";
      inputs.sisho.follows = "";
    };
    catasto = {
      url = "github:tomet-lang/fs-catasto";
      inputs.tomet.follows = "";
      inputs.twrit.follows = "";
      inputs.tomet-shelf.follows = "";
      inputs.sisho.follows = "";
    };
    tetch = {
      url = "github:tomet-lang/tetch";
      inputs.sisho.follows = "";
      inputs.tomet.follows = "";
      inputs.twrit.follows = "";
      inputs.tomet-shelf.follows = "";
    };
  };

  outputs = inputs: import ./nix inputs;
}
