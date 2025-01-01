{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    toolchain = fenix.packages.${system}.complete.toolchain;

    nativeBuildInputs = [
      toolchain
      pkgs.cmake
      pkgs.pkg-config
    ];
    buildInputs = [pkgs.openssl.dev];
  in {
    devShell.${system} = pkgs.mkShell {
      name = "starship";
      inherit nativeBuildInputs buildInputs;
      packages = with pkgs; [
        cargo-nextest
      ];
    };
  };
}
