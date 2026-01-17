{
  description = "A simple Wayland music player written in Rust";
  inputs.nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forEachSupportedSystem =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            pkgs = import nixpkgs { inherit system; };
          }
        );

    in
    {
      packages = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "Waitify";
            version = "0.1.0-git";
            src = self;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = with pkgs; [
              rustPlatform.bindgenHook
              pkg-config
            ];
            cargoBuildType = "debug";
            cargoCheckType = "debug";

            dontStrip = true;
          };
        }
      );
      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              pkg-config
              libxkbcommon
              cargo
              rustc
              rust-analyzer
              rustfmt
              labwc
              pam
              clippy
              alsa-lib
              libGL
            ];
            
            buildInputs = with pkgs; [
              libxkbcommon
              alsa-lib
              pam
              libGL
            ];
            
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
              pkgs.libGL
              pkgs.libxkbcommon
              pkgs.wayland
              pkgs.pam
              pkgs.alsa-lib
            ];
            
            shellHook = ''
              export RUST_SRC_PATH="${pkgs.rustPlatform.rustLibSrc}"
              
              export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" [ 
                pkgs.libxkbcommon 
                pkgs.wayland 
                pkgs.libGL 
                pkgs.pam
                pkgs.alsa-lib
              ]}"
              
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [ 
                pkgs.libxkbcommon 
                pkgs.libGL 
                pkgs.wayland 
                pkgs.pam
              ]}"
            '';
          };
        }
      );
    };
}
