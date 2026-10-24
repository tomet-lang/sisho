{
  pkgs,
  mkShell,
  fenix,
  tomet,
  tomet-lsp,
  twrit,
  catasto,
  sisho-desk,
  ...
}:
let
  rustToolchain = fenix.combine [
    (fenix.stable.withComponents [
      "cargo"
      "clippy"
      "rustc"
      "rust-src"
      "rustfmt"
      "rust-analyzer"
    ])
  ];
in
mkShell {
  buildInputs = with pkgs; [
    tomet
    tomet-lsp
    twrit
    catasto
    sisho-desk
    just

    #= Build
    pkg-config

    #= Rust
    rustToolchain
    cargo-edit
    cargo-outdated
    cargo-nextest
  ];

  shellHook = ''
    echo "🦀📚 Rust Tomet"
  '';
}
