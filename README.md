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
  create-schema     Create the schema into the DB
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

Download the latest release from the [releases page](https://github.com/to_define/cdir/releases).

Next, extract the archive, run the `install.sh` script located in the extracted directory, and follow the on-screen
instructions.

## Customization

Several aspects of `cdir` can be customized to fit your needs.
You can report to the configuration file for the available options.
The path to the configuration file can be found using:

```aiignore
$ cdir config-file
```

Colors are customizable, which is mandatory if you use a dark terminal theme.

To do so, you need to edit the configuration file and, for instance, add

```yaml
colors:
  date: "#80c0ff"
  path: "#ffffff"
  shortcut_name: "#70eeb0"
```

## Going further

Using directly the `cdir` cli provides more features, such as importing shortcuts for a file:

To do so, you need to have a YAML file with a list of shortcuts defined by a `name` and a `path`.

Example:

```yaml
- name: t
  path: /tmp
- ...
```

Then you can import it using the `cdir import` command:

```
$ cdir import-shortcuts /path/to/shortcuts.yaml
```

You can also delete a shortcut using the `cdir delete-shortcut` command:

```
$ cdir delete-shortcut myshortcut
```

## License

This project is licensed under the Apache License 2.0.
See the [LICENSE](LICENSE) file for details.