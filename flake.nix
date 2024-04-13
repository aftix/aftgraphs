{
  description = "Rust crate for creating physics simulations targeting x86, WASM, and HEVC rendered outputs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    system = "x86_64-linux"; # Building only supported on x64 linux
    overlays = [(import rust-overlay)];
    pkgs = import nixpkgs {inherit system overlays;};
    rustToolchainCfg = builtins.fromTOML (builtins.readFile ./rust-toolchain.toml);
    rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
    llvmPkgs = pkgs.llvmPackages_18;
    nativeBuildInputs = with pkgs; [pkg-config];
    devDependencies = with pkgs; [cargo-nextest tokio-console];
    buildDependencies = with pkgs; [
      x264
      rustToolchain
      sccache
      wasm-bindgen-cli
      clang
      lldb_18
    ];
    libclangPath = pkgs.lib.makeLibraryPath [llvmPkgs.libclang.lib];
  in {
    formatter."${system}" = pkgs.alejandra;

    devShells."${system}".default = let
      inherit nativeBuildInputs;
    in
      pkgs.mkShell {
        inherit nativeBuildInputs;
        buildInputs = buildDependencies ++ devDependencies;

        RUSTC_VERSION = rustToolchainCfg.toolchain.channel;
        RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
        LIBCLANG_PATH = libclangPath;

        shellHook = ''
          export PATH="$PATH:''${CARGO_HOME:-~/.cargo}/bin"
          export PATH="$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/"
          rustup override set ${rustToolchainCfg.toolchain.channel}
          ${builtins.concatStringsSep "\n" (map (t: "rustup target add ${t}") rustToolchainCfg.toolchain.targets)}
        '';
      };
  };
}
