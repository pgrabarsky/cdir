#!/bin/sh
# cdir installer script (for user specific manual installation from a tarball)
# This script installs the cdir binary to $HOME/.local/bin (or INSTALL_BIN_DIR if set),
# and runs the init.sh script for additional setup.

set -e

DEFAULT_INSTALL_BIN_DIR="$HOME/.local/bin"

PKG_DIR="$(cd "$(dirname "$0")"; pwd)"

if [ -f "$(cd "$(dirname "$0")"; pwd)/cdir" ]; then
    BIN_DIR="$(cd "$(dirname "$0")"; pwd)"
else
    BIN_DIR="$(cd "$(dirname "$0")"; pwd)/target/release"
fi

install() {
    local _install_bin_dir
    _install_bin_dir=${INSTALL_BIN_DIR:-}

    if [ -z "${_install_bin_dir}" ]; then
        _install_bin_dir=$DEFAULT_INSTALL_BIN_DIR
    fi

    # Check if binary exists
    if [ ! -f "$PKG_DIR/cdir" ]; then
        echo "Error: cdir binary not found in $PKG_DIR"
        return 1
    fi

    echo "Installing the binary 'cdir' into: $_install_bin_dir..."
    if [ ! -d "$_install_bin_dir" ]; then
        mkdir -p "$_install_bin_dir" || {
            echo "Error: Failed to create directory $_install_bin_dir"
            return 1
        }
    fi

    cp "$PKG_DIR/cdir" "$_install_bin_dir/" || {
        echo "Error: Failed to copy binary to $_install_bin_dir"
        return 1
    }
    chmod +x "$_install_bin_dir/cdir" || {
        echo "Error: Failed to set executable permission"
        return 1
    }

    # Copy shell functions file
    echo "Installing the shell functions 'cdir_funcs.sh' into: $_install_bin_dir..."
    if [ -f "$PKG_DIR/cdir_funcs.sh" ]; then
        cp "$PKG_DIR/cdir_funcs.sh" "$_install_bin_dir/" || {
            echo "Error: Failed to copy cdir_funcs.sh"
            return 1
        }
    else
        echo "Warning: cdir_funcs.sh not found in $PKG_DIR"
    fi

    echo "done"

    # Check if bin directory is in PATH
    case ":$PATH:" in
        *":$_install_bin_dir:"*) ;;
        *)
            echo ""
            echo "WARNING: $_install_bin_dir is not in your PATH."
            echo "You may need to add it to your shell configuration file (export PATH=\$PATH:$_install_bin_dir)."
            ;;
    esac


}

echo "#############################"
echo "#      cdir installer       #"
echo "#############################"
echo
install || exit 1