{
  description = "A basic flake for my Bevy Game";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-flake.url = "github:juspay/rust-flake";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        # Optional: use external flake logic, e.g.
        # inputs.foo.flakeModules.default
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs
      ];
      flake = {
        # Put your original flake attributes here.
      };
      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin"];
      perSystem = {
        # self',
        pkgs,
        ...
      }: let
        rustNightly = pkgs.rust-bin.nightly.latest.default.override {
          extensions = ["rust-src" "clippy" "rustfmt"];
          targets = ["wasm32-unknown-unknown"];
        };
      in {
        devShells.default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            openssl
            trunk
            wasm-pack
            rustNightly
            clippy
            rust-analyzer
            pkg-config
            llvmPackages.bintools
            nodejs_22 # for the web/ landing site (matches the CI setup-node version)
            samply # sampling profiler for the scenario-dispatch benchmarks (task 20260714-083331)
          ];

          buildInputs = with pkgs; [
            udev
            alsa-lib-with-plugins
            vulkan-loader
            libx11
            libxcursor
            libxi
            libxrandr # To use the x11 feature
            libxkbcommon
            wayland # To use the wayland feature
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          RUST_BACKTRACE = 1;

          RUST_SRC_PATH = "${rustNightly}/lib/rustlib/src/rust/library";
        };
      };
    };
}
