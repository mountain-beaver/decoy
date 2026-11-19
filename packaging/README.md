# Packaging

Recipes for package catalogs. They install the files from a published release.

- `aur/`: the `decoy-bin` package for the Arch User Repository. After a release, set `pkgver` and the checksums, then run `makepkg --printsrcinfo > .SRCINFO`.
- `winget/`: the manifest for the Windows Package Manager. It goes to `manifests/m/mountain-beaver/Decoy/<version>/` in `microsoft/winget-pkgs`.
