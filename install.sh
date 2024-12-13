#!/bin/sh

DEFAULT_INSTALL_BIN_DIR="$HOME/.local/bin"
DEFAULT_INSTALL_CONFIG_DIR="$HOME/.config/cdir"
DEFAULT_INSTALL_DATA_DIR="$HOME/.local/share/cdir"

PKG_DIR="$(cd "$(dirname "$0")"; pwd)"

install() {
    local _install_bin_dir
    local _install_config_dir
    local _install_data_dir
    
    _install_bin_dir=${INSTALL_BIN_DIR:-}
    _install_config_dir=${INSTALL_CONFIG_DIR:-}
    _install_data_dir=${INSTALL_DATA_DIR:-}

    if [ -z "${_install_bin_dir}" ]; then
        _install_bin_dir=$DEFAULT_INSTALL_BIN_DIR
    fi
    if [ -z "${_install_config_dir:-}" ]; then
        if [ ! -z "${XDG_CONFIG_HOME:-}" ]; then
            _install_config_dir=${XDG_CONFIG_HOME:-}/cdir
        else
            _install_config_dir=$DEFAULT_INSTALL_CONFIG_DIR
        fi
    fi
    if [ -z "${_install_data_dir:-}" ]; then
        if [ ! -z "${XDG_DATA_HOME:-}" ]; then
            _install_data_dir=${XDG_DATA_HOME:-}/cdir
        else
            _install_data_dir=$DEFAULT_INSTALL_DATA_DIR
        fi
    fi

    echo "#############################"
	echo "Installing the binary into: $_install_bin_dir..."
    if [ ! -d "$_install_bin_dir" ]; then
        mkdir -p "$_install_bin_dir"
    fi
    cp "$PKG_DIR/cdir" "$_install_bin_dir/"
    sed -e "s|__BIN_PATH__|$_install_bin_dir|g"  -e "s|__CONFIG_PATH__|$_install_config_dir|g" -e "s|__DATA_PATH__|$_install_data_dir|g" "$PKG_DIR/templates/cdir_funcs.sh" > "$_install_bin_dir/cdir_funcs.sh"
    echo "done"

    echo "#############################"
    echo "Installing the configuration into: $_install_config_dir..."
    if [ ! -d "$_install_config_dir" ]; then
        mkdir -p "$_install_config_dir"
    fi
    if [ ! -e "$_install_config_dir/config.yaml" ]; then
        sed -e "s|__CONFIG_PATH__|$_install_config_dir|g" -e "s|__DATA_PATH__|$_install_data_dir|g" "$PKG_DIR/templates/config.yaml" > "$_install_config_dir/config.yaml"
    else
        echo "$_install_config_dir/config.yaml file already exists, skipping"
    fi
    if [ ! -e "$_install_config_dir/config.yaml" ]; then
        sed -e "s|__CONFIG_PATH__|$_install_config_dir|g" -e "s|__DATA_PATH__|$_install_data_dir|g" "$PKG_DIR/templates/log4rs.yaml" >"$_install_config_dir/log4rs.yaml"
    else
        echo "$_install_config_dir/log4rs.yaml file already exists, skipping"
    fi
    echo "done"

    echo "#############################"
    echo "Installing the data into: $_install_data_dir..."
    if [ ! -d "$_install_data_dir" ]; then
        mkdir -p "$_install_data_dir"
    fi
    echo "done"

    echo "#############################"
    echo "# Action(s):"
    
    if [ "$_install_config_dir" != "$DEFAULT_INSTALL_CONFIG_DIR" ]; then
    echo "* Add the following variable to your environment if not already done:"
    echo "export CDIR_CONFIG=$_install_config_dir/config.yaml"
    fi

    echo "* Add $_install_bin_dir to your path if not already done:"
    echo "export PATH=\$PATH:$_install_bin_dir"

    echo "* Add the following into you shell launch script (e.g. .zshrc):"
    echo "source $_install_bin_dir/cdir_funcs.sh"
}

echo "#############################"
echo "#      cdir installer       # "
echo "#############################"
echo
install || exit 1