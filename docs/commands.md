# Commands

Let's review the commands that are available from your terminal when you are outside of the UI.

## Most used commands

There are three main commands from the terminal:

1. Launch the `cdir` UI:
  ```
  $ c
  ```

1. Create a named shortcut to the current directory:
  ```
  $ p myshortcut "optional description"
  ```

1. Jump into a directory by shortcut name:
  ```
  $ c myshortcut-name
  ```

## Others

You can discover other commands using `cdir --help`:

```
$ cdir --help
cdir helps you to switch quickly and easily between directories

Usage: cdir [OPTIONS] [COMMAND]

Commands:
  gui                  Launch the GUI
  config-file          Print the path to the configuration file
  add-path             Add a directory path
  add-shortcut         Add a shortcut
  delete-shortcut      Delete a shortcut
  print-shortcut       Print a shortcut
  lasts                Print last paths
  pretty-print-path    Pretty print a path using shortcuts
  import-paths         Import a path file
  export-paths         Export paths to a YAML file
  import-path-history  Import path history file
  export-path-history  Export path history to a YAML file
  import-shortcuts     Import a shortcuts file
  export-shortcuts     Export shortcuts to a YAML file
  help                 Print this message or the help of the given subcommand(s)

Options:
  -c, --config-file <config_file>  Path to the configuration file
  -h, --help                       Print help
  -V, --version                    Print version
```

You can refer to the following sections for more details:

* Import/Export commands: [Data Import & Export](importing_shortcuts.md)

* `pretty-print-path`: [Shell prompt](prompt.md)
