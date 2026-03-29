{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);

      # Shared pkgs instantiation — carries rust-overlay for devShell,
      # buildRustPackage uses nixpkgs's own rustc so the overlay is a no-op there.
      mkPkgs = system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
    in {
      packages = forAllSystems (system:
        let
          pkgs = mkPkgs system;
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "nexus-gen";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "nexus-gen IPC topology code generator";
              mainProgram = "nexus-gen";
            };
          };
        });

      devShells = forAllSystems (system:
        let
          pkgs = mkPkgs system;
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" ];
          };
        in {
          default = pkgs.mkShell {
            buildInputs = [
              rust
              pkgs.pkg-config
              pkgs.gcc
            ];
          };
        });
    };
}
