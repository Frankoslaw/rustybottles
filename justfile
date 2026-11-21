# Build rustybottles in release mode.
build:
    cargo build --release

# Remove compiled artifacts.
clean:
    cargo clean

# Install rustybottles into INSTALL_DIR (defaults to ~/.local/bin).
# Confirm before replacing an existing executable.
install: build
    #!/usr/bin/env bash
    install_dir="${INSTALL_DIR:-$HOME/.local/bin}"
    mkdir -p "$install_dir"
    if [ -e "$install_dir/rustybottles" ] || [ -L "$install_dir/rustybottles" ]; then
        printf 'Warning: %s already exists and will be overwritten.\n' "$install_dir/rustybottles"
        read -r -p 'Continue? [y/N] ' answer
        case "$answer" in
            y|Y|yes|YES) ;;
            *) echo 'Installation cancelled.'; exit 1 ;;
        esac
    fi
    install -m 755 target/release/rustybottles "$install_dir/rustybottles"
    echo "Installed rustybottles to $install_dir"

# Install completions for the detected shell, or COMPLETION_SHELL.
install-completions: build
    #!/usr/bin/env bash
    shell="${COMPLETION_SHELL:-${SHELL##*/}}"
    case "$shell" in
        bash)
            completion_dir="${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions"
            completion_file="$completion_dir/rustybottles"
            ;;
        fish)
            completion_dir="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
            completion_file="$completion_dir/rustybottles.fish"
            ;;
        zsh)
            completion_dir="${ZSH_COMPLETION_DIR:-$HOME/.zfunc}"
            completion_file="$completion_dir/_rustybottles"
            ;;
        *)
            echo "Unsupported shell '$shell'. Set COMPLETION_SHELL to bash, fish, or zsh." >&2
            exit 1
            ;;
    esac
    mkdir -p "$completion_dir"
    if [ -e "$completion_file" ] || [ -L "$completion_file" ]; then
        printf 'Warning: %s already exists and will be overwritten.\n' "$completion_file"
        read -r -p 'Continue? [y/N] ' answer
        case "$answer" in
            y|Y|yes|YES) ;;
            *) echo 'Completion installation cancelled.'; exit 1 ;;
        esac
    fi
    target/release/rustybottles --generate-completion "$shell" > "$completion_file"
    echo "Installed $shell completions to $completion_dir"
    if [ "$shell" = zsh ]; then
        echo 'For zsh, ensure this directory is on fpath (for example: fpath=(~/.zfunc $fpath)).'
    fi

# Remove rustybottles from INSTALL_DIR.
uninstall:
    rm -f "${INSTALL_DIR:-$HOME/.local/bin}/rustybottles"
