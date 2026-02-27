{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            clippy
            rustfmt
            pkg-config
            overmind
            tmux
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          # Runtime libs for eframe/wgpu
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
            vulkan-loader
            libxkbcommon
            wayland
            libGL
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ]);
        };
      });
}
