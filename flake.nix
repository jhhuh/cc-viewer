{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        runtimeLibs = with pkgs; [
          vulkan-loader
          libxkbcommon
          wayland
          libGL
          mesa
          libx11
          libxcursor
          libxi
          libxrandr
        ];

        runtimeLibPath = pkgs.lib.makeLibraryPath runtimeLibs;

        cc-viewer-unwrapped = pkgs.rustPlatform.buildRustPackage {
          pname = "cc-viewer";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            wayland
            libxkbcommon
          ];
        };

        cc-viewer = pkgs.writeShellScriptBin "cc-viewer" ''
          export LD_LIBRARY_PATH="${runtimeLibPath}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
          exec ${cc-viewer-unwrapped}/bin/cc-viewer "$@"
        '';

        docs = pkgs.stdenv.mkDerivation {
          pname = "cc-viewer-docs";
          version = "0.1.0";
          src = ./docs;

          nativeBuildInputs = [ pkgs.mdbook ];

          buildPhase = ''
            mdbook build
          '';

          installPhase = ''
            cp -r book $out
          '';
        };

        serve-docs = pkgs.writeShellScriptBin "cc-viewer-docs" ''
          echo "Serving docs at http://localhost:3000"
          ${pkgs.python3}/bin/python3 -m http.server 3000 -b 127.0.0.1 -d ${docs}
        '';

        take-screenshots = pkgs.writeShellScriptBin "cc-viewer-screenshots" ''
          export PATH="${pkgs.lib.makeBinPath [
            cc-viewer
            pkgs.xorg.xorgserver
            pkgs.xdotool
            pkgs.imagemagick
            pkgs.coreutils
          ]}:$PATH"
          export LD_LIBRARY_PATH="${runtimeLibPath}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
          exec ${./scripts/screenshots.sh}
        '';
      in {
        packages = {
          default = cc-viewer;
          inherit cc-viewer docs;
          unwrapped = cc-viewer-unwrapped;
        };

        apps = {
          default = {
            type = "app";
            program = "${cc-viewer}/bin/cc-viewer";
          };
          docs = {
            type = "app";
            program = "${serve-docs}/bin/cc-viewer-docs";
          };
          screenshots = {
            type = "app";
            program = "${take-screenshots}/bin/cc-viewer-screenshots";
          };
        };

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
            mdbook
            gh
            xvfb-run
            xorg.xorgserver
            imagemagick
            xdotool
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          LD_LIBRARY_PATH = runtimeLibPath;
        };
      });
}
