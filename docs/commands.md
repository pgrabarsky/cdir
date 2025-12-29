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
  gui               Launch the GUI
  config-file       Print the path to the configuration file
  add-path          Add a directory path
  import-paths      Import a path file
  add-shortcut      Add a shortcut
  delete-shortcut   Delete a shortcut
  print-shortcut    Print a shortcut
  import-shortcuts  Import a shortcuts file
  lasts             Print last paths
  pretty-print-path  Pretty print a path using shortcuts  
  help              Print this message or the help of the given subcommand(s)

Options:
  -c, --config-file <config_file>  Path to the configuration file
  -h, --help                       Print help
  -V, --version                    Print version
```

Some of these commands are detailed into other part of this documentation.