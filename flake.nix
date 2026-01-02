{
  description = "Rustine dev shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: let
    systems = [ "x86_64-linux" "aarch64-linux" ];
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
  in {
    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          pkg-config
          cmake
          ninja
          clang
          lld
          git
          python3
          rustc
          rustfmt
          cargo
          rust-analyzer

          # Graphics / windowing
          glfw
          vulkan-headers
          vulkan-loader
          vulkan-validation-layers
          mesa
          xorg.libX11
          xorg.libXrandr
          xorg.libXcursor
          xorg.libXi
          xorg.libXinerama
          libxkbcommon
          systemd
          wayland
          wayland-protocols
          wayland-scanner
        ];

        shellHook = ''
          export RUST_BACKTRACE=1
        '';
      };
    });
  };
}
