{
  pkgs ? import (builtins.getFlake "nixpkgs") { },
}:
let
  # Loaded at runtime with `dlopen`, so they have to be on `LD_LIBRARY_PATH`
  libDeps = with pkgs; [
    libxkbcommon
    vulkan-loader
    wayland
    libx11
    libxcursor
    libxi
  ];
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    rustc
  ];
  buildInputs = libDeps;

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libDeps;
}
