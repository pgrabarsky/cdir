# Integration with the promp

You can benefit from `cdir` shortcuts and pretty printing to use them into your shell prompt.

For instance, let's assume you have defined the following shortcut `fe` associated to `~/microservices-demo/src/frontend`:

![shortcuts](pictures/shortcuts.png)

If your current directory is `~/microservices-demo/src/frontend`, then the path in your prompt will look like `[fe]`:

![prompt](pictures/prompt.png)

This solves in an elegant way the problem of have very long paths displayed into you prompt.

This is based on the `pretty-print-path` command that you can use into the configuration of your prompt manager.

Here is an explanation of this command, and then examples of how to use it into `zsh`, `bash` and [starship.rs](https://starship.rs/).

## The `pretty-print-path` command

### Basics

You can invoke `cdir` to pretty print a path in the same way as it is displayed into the _directory history_ view.

To do so, you just have to invoke the `pretty-print-path` command with the path, e.g.:

```bash
# Let's assume that the shortcut 'shortcut' = '/the/path/to'
$ cdir pretty-print-path "/the/path/to/pretty/print"
[shortcut]/pretty/print
```

As shown herebefore, a shortcut substitution has been performed.

The theme colors and styles are also used to pretty print the path exactly in the same way as it is displayed into the _directory history_ view.

### Additional options

The `pretty-print-path` command accept two additional and optional parameters:

First, the color and styles can be removed from the output in order to get a simple string of characters.
To do so, just add the `false` parameter to the command.

```bash
$ cdir pretty-print-path "/the/path/to/pretty/print" false
```

Second, you can limit the max size of the string of characters by providing a numerical value.
For instance, to limit the size to 40 characters:

```bash
$ cdir pretty-print-path "/the/path/to/pretty/print" true (or false) 40
```

## Customizing the zsh prompt

You can integrate `cdir` directly into your zsh prompt.

Add the following to your `.zshrc` file:

```zsh
setopt PROMPT_SUBST
function cdir_prompt() {
	cdir pretty-print-path "$PWD"
}
PROMPT='$(cdir_prompt) \$ '
```

This will display the pretty-printed current directory in your prompt, including shortcut substitutions and theme colors.

After editing `.zshrc`, reload your shell or run `source ~/.zshrc` to apply the changes.

## Customizing the bash prompt

You can also integrate `cdir` into your bash prompt. Add the following to your `.bashrc` file:

```bash
function cdir_prompt() {
	cdir pretty-print-path "$PWD"
}
export PS1='$(cdir_prompt) \$ '
```

This will display the pretty-printed current directory in your prompt, including shortcut substitutions and theme colors.

After editing `.bashrc`, reload your shell or run `source ~/.bashrc` to apply the changes.

## Integration with [starship.rs](https://starship.rs/)

In order to use `cdir` with [starship.rs](https://starship.rs/), the [Custom commands](https://starship.rs/config/#custom-commands ) feature should be used.

Let's define a custom command into the `starship.toml` configuration file:
```toml
[custom.cdir]
command = 'cdir pretty-print-path "`pwd`"'
when = true
```

Then, adjust the `format` parameter to replace the `${directory}` directive by the `${custom.cdir}` one, i.e.:

From:
```toml
format="""
...
${directory}\
..."""
```

To:
```toml
format="""
...
${custom.cdir}\
..."""
```