# Installation

The installation process is:

* Installing the `cdir` binary and a shell script;

* Creating the configuration files (during the first installation);

* Setup up your shell rc file to source the shell script.

## With Homebrew (macOS only)

This is the recommanded way to install `cdir` on macOS as it is more automated and it also provides automatic updates.

1. Setup the AmadeusITGroup tap:

    ```
    brew tap AmadeusITGroup/tap
    ```

1. Install cdir:

    ```
    brew install cdir
    ```

1. If it is the first time you use `cdir`, you have to call `cdir` once
    to initialize the configuration:

    ```
    $ cdir
    → Initializing the configuration...
    → Creating the default configuration file "/Users/<user>/.config/cdir/config.yaml"
    → Creating data directory "/Users/<user>/Library/Application Support/cdir"
    → Creating the cdir shell file "/Users/<user>/.cdirsh"
    → Adding source line to "~/.zshrc"
    ✓ Configuration is ready. Please restart your shell or run 'source ~/.cdirsh' to apply the changes.
    Use the 'c' shell alias to launch the GUI.
    Use --help to see available commands.
    Documentation is available at https://github.com/AmadeusITGroup/cdir
    ```

At this point the setup is done. 
Either restart your shell or run `source ~/.cdirsh` to apply the changes.<br>

Then, you can use the `c` shell alias (see [Commands](commands.md)) to launch the GUI, and the `p` alias to create shortcuts.

## From the tarball (Linux or macOS)

1. Download the latest release from the <a href="https://github.com/AmadeusITGroup/cdir/releases">releases page</a>.

1. Extract the archive:

    ```
    $ tar xzf cdir-aarch64-apple-darwin.tar.gz
    ```

1. Go to the extracted directory and run the installer:

    ```
    $ cd cdir-aarch64-apple-darwin
    $ ./install.sh
    #############################
    #      cdir installer       #
    #############################

    Installing the binary 'cdir' into: /home/<user>/.local/bin...
    Installing the shell functions 'cdir_funcs.sh' into: /home/<user>/.local/bin...
    done
    ```


1. If needed, add the installation directory to your `PATH`.

    ```
    $ export PATH=$PATH:/root/.local/bin # update you shell profile if needed...
    ```

1. If it is the first time you use `cdir`, you have to call `cdir` once as described in the section for macOS here above.

    ```
    $ cdir
    → Initializing the configuration...
    ...
    ```

At this point the setup is done. 
Either restart your shell or run `source ~/.cdirsh` to apply the changes.<br>

Then, you can use the `c` shell alias (see [Commands](commands.md)) to launch the GUI, and the `p` alias to create shortcuts.

