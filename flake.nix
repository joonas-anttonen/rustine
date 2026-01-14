{
  description = "dev-shell: Rustine";

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
          meson
          ninja
          rustc
          rustfmt
          cargo
          rust-analyzer

          valgrind

          renderdoc

          vulkan-headers
          vulkan-loader
          vulkan-validation-layers

          systemd
          libffi
          wayland
          wayland-protocols
          wayland-scanner
          libxkbcommon

          directx-shader-compiler
          directx-shader-compiler.dev

          ffmpeg
          ffmpeg.dev
      
        ];

        shellHook = ''
          export RUST_BACKTRACE=1
          export DXC_INCLUDE_DIR="${pkgs.directx-shader-compiler.dev}/include/dxc"
          export DXC_LIB_DIR="${pkgs.directx-shader-compiler}/lib"
        '';
      };
    });
  };
}
