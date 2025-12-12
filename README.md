# cdir

cdir allows you to quickly navigate to a directory recorded into your history.

When you use the `cd` command in your terminal, `cdir` records the directory into its database.
Then you can use the graphical user interface to go back to it later.
The GUI has text filtering capabilities, and it shows the date of the last time you went to it.

cdir also supports creating shortcuts for frequently used directories.

<p align="center">
  <img src="doc/demo.gif" alt="animated" />
</p>

## Features

* Records your directory history
* Quickly navigate to previously visited directories with a console UI
* Directory shortcuts
* Text search for directories and shortcuts
* Supports multiple shells (zsh, bash)
* Import predefined shortcuts from a YAML file
* Customizable colors
* Supports multiple platforms (Linux, macOS)

## Commands

There are three main commands from the terminal:

Open the cdir ui:

```
$ c
```

Create a named shortcut to the current directory

```
$ p myshortcut
```

You can also use the `c` command to go directly to a directory by using a shortcut name:

```
$ c myshortcut
```

Some additional features are available using the `cdir` command:

```
$ cdir --help
cdir helps you to switch quickly and easily between directories

Usage: cdir [OPTIONS] [COMMAND]

Commands:
  gui               Launch the GUI
  config-file       Print the path to the configuration file
  add-path          Add a directory path
  import-paths      Import a path file
  add-shortcut      Add a shortcut
  delete-shortcut   Delete a shortcut
  print-shortcut    Print a shortcut
  import-shortcuts  Import a shortcuts file
  lasts             Print last paths
  help              Print this message or the help of the given subcommand(s)

Options:
  -c, --config-file <CONFIG_FILE>  Path to the configuration file
  -h, --help                       Print help
  -V, --version                    Print version
```

## Navigating the UI

You can open the UI by typing `c` in your terminal.

There are three views:

* The directory history view, which shows the list of previously visited directories, sorted by most recent visit. It
  also shows the date of the last visit.

* The shortcuts view, which shows the list of defined shortcuts.

* The help view, which shows the available commands.

Use `tab` to switch between the two first views, and `ctrl + h` to go to the help view.

Then you can navigate using the following keys:

* Use `enter` to exit the GUI and go into the selected directory;

* Use `esc` ot `ctrl + q` to simply exit and stay in the current directory;

* Use the `up` and `down` arrow keys to select a directory (`shift` for bigger jumps);

* Use `page up` and `page down` to scroll through the list by page;

* Use `home` to go to the most recent directory in the history (the top);

* Use `ctrl + a` to see the full directory path in the directory history view.

* Use `ctrl + d` to delete the selected entry.

You can type a free text to filter the directory history or shortcuts.

Tip: You can use `ctrl+a` to see the full directory path in the directory history view.

## Installation

### With [Homebrew](https://brew.sh/) on MacOS

This is the easiest way to install `cdir` and get automatic updates on macOS if you have Homebrew installed.

Setup the AmadeusITGroup tap:

```
$ brew tap AmadeusITGroup/tap
```

Then install cdir:

```
$ brew install cdir
```

If it is the first time you use `cdir`, you have the perform the
following step to initialize the configuration:

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

### From the tarball

#### 1. Download the tarball

Download the latest release matching your architecture from
the [releases page](https://github.com/AmadeusITGroup/cdir/releases).

Next, extract the archive, e.g.:

```
$ tar xzf cdir-aarch64-apple-darwin.tar.gz
```

#### 2. Install the executable and shell functions

Go to the extracted directory and launch the `install.sh` script:

```
$ cd cdir-aarch64-apple-darwin
$ ./install.sh
#############################
#      cdir installer       #
#############################

Installing the binary 'cdir' into: /Users/<user>/.local/bin...
Installing the shell functions 'cdir_funcs.sh' into: /Users/<user>/.local/bin...
done

WARNING: /Users/<user>/.local/bin is not in your PATH.
You may need to add it to your shell configuration file (export PATH=$PATH:/Users/<user>/.local/bin).
```

If you can see the warning about the PATH, you need to add the installation directory to your PATH environment variable
(typically into your `.zshrc` or `.bashrc` file).

, run the `install.sh` script located in the extracted directory, and follow the on-screen
instructions.

#### 3. Initialize the configuration

When the `cdir` binary can be found into your PATH, and if it is the first time you use `cdir`, you have the perform the
following step to initialize the configuration:

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

The next time you open a terminal, you can use the `c` and `p` commands and start using `cdir`.

## Configuration

### Path to the configuration file

Several aspects of `cdir` can be customized to fit your needs.
You can report to the configuration file for the available options.
The path to the configuration file can be found using:

```aiignore
$ cdir config-file
```

### Colors

Default colors have been chosen to provide a good contrast on both light and dark terminal themes.
But they can be customized.

To do so, you need to edit the configuration file.
For instance, you can uncomment the light theme proposal included in the default configuration file:

```yaml
colors:
  date: "#808080"
  path: "#000000"
  highlight: "#ffe680"
  shortcut_name: "#008800"
  description: "#808080"
```

The available options are:

* `date`: the color of the date in the directory history view

* `path`: the color of the path in the directory history view

* `highlight`: the color of the highlighting for directory selection

* `shortcut_name`: the color of the shortcut name

* `description`: the color of the shortcut description

* `header_fg`: the color of the header foreground

* `header_bg`: the color of the header background

### Date format

The date format can be customized using the `date_format` option in the configuration file.
Report to [the chrono package doc](https://docs.rs/chrono/0.4.38/chrono/format/strftime/index.html) for the formating
options.

```yaml
date_format: "%d-%b-%y %H:%M" 
```

### Path to the database

The path to the database can be set using the `db_path` option in the configuration file.
Beware that it should include the filename, for instance:

```yaml
db_path: "/home/user/.local/share/cdir/development.db"
```

### Path to the log configuration file

The path to the log configuration file can be set using the `log_config_path` option in the configuration file.

## Importing shortcuts

cdir supports importing shortcuts for a YAML file:

To do so, the file should contain the list of shortcuts defined by a `name` and a `path`.

Example:

```yaml
- name: t1
  path: /tmp1
- name: t2
  path: /tmp2
  description: Temporary directory 2
```

Then you can import it using the `cdir import` command:

```
$ cdir import-shortcuts /path/to/shortcuts.yaml
```

You can also delete a shortcut using the `cdir delete-shortcut` command:

```
$ cdir delete-shortcut myshortcut
```

## Logging

By default, cdir logs are configured in a file name `log4rs.yaml` located in the same directory as the configuration
file.
It can be adjusted, noticeably to change the log level.

cdir uses the [log4rs](https://docs.rs/log4rs/latest/log4rs/) crate for logging.
The documentation for the file configuration is [here](https://docs.rs/log4rs/latest/log4rs/#configuration).

## License

This project is licensed under the Apache License 2.0.
See the [LICENSE](LICENSE) file for details.