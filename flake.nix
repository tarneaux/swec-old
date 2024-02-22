{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, nixpkgs-unstable }: let
    pkgs = nixpkgs.legacyPackages.x86_64-linux;
    pkgs-unstable = nixpkgs-unstable.legacyPackages.x86_64-linux;
  in {
    devShells.x86_64-linux.default = pkgs.mkShell {
      packages = [
        pkgs.openssl
        pkgs.pkg-config
        pkgs-unstable.cargo
        pkgs-unstable.rustc
        pkgs-unstable.bacon
        pkgs-unstable.rust-analyzer
        pkgs-unstable.rustfmt
        pkgs-unstable.clippy
      ];
      shellHook = ''
        export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig";
      '';
    };
  };
}
