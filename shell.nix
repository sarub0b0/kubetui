{pkgs ? import<nixpkgs>{}}:

pkgs.mkShell {
  buildInputs = [
    pkgs.xorg.libxcb
    pkgs.openssl
    pkgs.pkgconfig
  ];
}
