{
  description = "Decoy: a small cross-platform Discord Rich Presence editor";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      # Libraries Tauri 2 links or loads through dlopen on Linux.
      libs = with pkgs; [
        at-spi2-atk
        atkmm
        cairo
        gdk-pixbuf
        glib
        gtk3
        harfbuzz
        librsvg
        libsoup_3
        pango
        webkitgtk_4_1
        openssl
        libayatana-appindicator
      ];
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages =
          (with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            cargo-tauri
            pkg-config
            gobject-introspection
            nodejs_24
            pnpm
          ])
          ++ libs;

        # The tray icon library is loaded through dlopen, so the path is needed at run time too.
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libs;
        # Without this the webview cannot open https (no GIO TLS module).
        GIO_MODULE_DIR = "${pkgs.glib-networking}/lib/gio/modules/";
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

        # Without GSettings schemas GDK reports DPI = -1 and WebKitGTK computes a negative window width:
        # the whole layout collapses. Regular distributions ship the schemas system-wide; in the nix shell they are provided explicitly.
        shellHook = ''
          export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS"
        '';
      };
    };
}
