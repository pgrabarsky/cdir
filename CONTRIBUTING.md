# Contributing to cdir

We would love for you to contribute to cdir and help make it even better than it is today!
As a contributor, here are the guidelines we would like you to follow:

- [Code of Conduct](#coc)
- [Licence](#License)
- [Building the project](#build)
- [Submission guidelines](#submission)

## <a name="coc"></a> Code of Conduct

Please read and follow our [Code of Conduct][coc].

## <a name="license"></a> License

This project is licensed under the Apache 2.0 License.
See the [LICENSE](LICENSE) file for details.

By contributing to this project, you agree that your contributions will be
licensed under the Apache 2.0 License.

## <a name="build"></a> Building the project

### Setup your environment

To build the project, you will need to have Rust installed on your machine.
You can install Rust by following the instructions on
the [official Rust website](https://www.rust-lang.org/tools/install).

### Building the binary

Then you can clone the repository and build the project using Cargo:

```bash
git clone https://github.com/AmadeusITGroup/cdir.git
cd cdir
cargo build
```

At this point, you have built the project in debug mode.

### Execution

On top of the unit test, when developing you will probably want to run the binary to test your changes:

With cdir there are a few extra considerations to take into account as it uses a `sqlite` database to store its
data, and it integrates with the shell.

#### Using a development database

The structure or the database of the development version may be different from the one of the release version.
If you have already cdir installed, you might want to use a different database for the development activities.

cdir uses a configuration file to determine the location of the database.
It will look for the configuration file in the following locations, in order:

* The path specified on the command line using the `--config-file` option;

* The environment variable `CDIR_CONFIG`;

* In the user's configuration directory, which is typically `~/.config/cdir/config.yaml`.

For development purposes, using the environment variable is probably the easiest way to specify a different
configuration file.

In the configuration file, you can specify the location of the database using the `db_path` entry, e.g.:

```
db_path: "/path/to/your/database.db"
```

#### Integrating with the shell

When installing cdir, it creates a shell script that needs to be sourced from `.zshrc` or `.bashrc` in order to work
properly (see [install.sh](install.sh)).

If you already have cdir installed, you can simply set the `CDIR_BIN` environment variable to point to the
binary you just built, e.g.:

```
export CDIR_BIN=`pwd`/target/debug/cdir
```

If you don't have cdir already installed, for development purposes you can create a similar script using the
`templates/cdir_func.sh` template, replacing the path to the binary with the path to the binary you just built. And then
source it from your
shell.


[coc]: ./CODE_OF_CONDUCT.md

## <a name="submission"></a> Submission guidelines

As the project is currently in its early stages, there are relatively few
contribution guidelines.
However, we do ask that you follow these general guidelines:

- Ensure your code adheres to the project's coding style and conventions.
- Write clear, concise commit messages that describe the changes you have made.
- Include tests for any new features or bug fixes you implement.
- Ensure that all tests pass before submitting a pull request.

To submit a contribution, please fork the repository and create a new branch for your changes.
Once you have made your changes, you can submit a pull request to the main repository.
We will review your pull request and provide feedback as necessary.

Thank you for considering contributing to cdir!