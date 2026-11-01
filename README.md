# rustybottles

`rustybottles` processes game archives and installs the executables it finds into [Bottles](https://usebottles.com/) bottles. It can extract nested archives, identify the main executable, create Bottle shortcuts and optionally add entries to the Bottles library.

## Usage

```text
rustybottles [OPTIONS] <PATH_OR_GLOB>...
```

The input can be a file, several files, or a glob pattern. Shell variables in paths are expanded by the program.

```sh
rustybottles game.zip
rustybottles ~/Downloads/*.zip
rustybottles game1.zip game2.7z
```

Options:

```text
-c, --config <FILE>    Use a configuration file other than rustybottles.toml
-h, --help             Print help
-V, --version          Print version
```

## Configuration

Start with the included example:

```sh
cp rustybottles.example.toml rustybottles.toml
```

At minimum, review `[app]` and `[bottles]` for your system. The example is configured for Bottles installed as a Flatpak; adjust `bottles.root`, `bottle`, and `install_path` to match your bottle.

The configuration has these sections:

- `[app]` controls the working directory, whether source files are copied or moved, cleanup, and deletion of the original.
- `[extract]` enables archive extraction. `extract_until` can be `"executable"`, `"noarchives"`, or `"maxdepth"`; `password` is optional.
- `[executable]` controls executable discovery. `search_pattern` defaults to `"*.exe"`; `prefer_longest` and `ignore_common` help select the main executable.
- `[bottles]` selects the Bottles root and bottle, installation directory, shortcut/library creation, force behavior, and RPG Maker compatibility handling.
- `[overrides.<name>]` detects special layouts by marker files and applies per-label bottle settings. The example includes `rpgmaker` and `unity` overrides.

Names used by `shortcut_tmpl` and `library_tmpl` support `{game_name}`, `{exe_name}`, `{folder_name}`, `{date}`, `{folder_hash}`, and `{exe_hash}`.

Run with the default configuration:

```sh
rustybottles path/to/game.zip
```

Or specify another configuration explicitly:

```sh
rustybottles --config ~/config/rustybottles.toml path/to/game.zip
```

Set `RUST_LOG=debug` for more detailed logs. The default log level is `info`.

## Nix

With Nix enabled, build or run the flake from a checkout:

```sh
nix run . -- --config rustybottles.toml path/to/game.zip
nix develop                 # enter the development shell
nix build                   # build the package
nix profile install .       # install rustybottles into your user profile
```

To use it in a NixOS flake, add this repository as an input and include its package in `environment.systemPackages`:

```nix
environment.systemPackages = [
  rustybottles.packages.${system}.default
];
```

The flake enables the unfree `unrar` dependency required by the Rust package.
