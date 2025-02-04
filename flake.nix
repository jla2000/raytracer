{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };
    in
    {
      devShells.${system} = rec {
        default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            rust-bin.stable.latest.minimal
          ];
          buildInputs = with pkgs; [
            vulkan-loader
            libGL
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            xorg.libXinerama
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
        };

        windows = pkgs.mkShellNoCC {
          nativeBuildInputs = with pkgs; [
            (rust-bin.stable.latest.minimal.override {
              targets = [ "x86_64-pc-windows-gnu" ];
            })
            pkgsCross.mingwW64.buildPackages.gcc
          ];
          CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
          CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-L native=${pkgs.pkgsCross.mingwW64.windows.pthreads}/lib";
        };
      };
    };
}
