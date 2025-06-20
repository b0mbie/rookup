# Rookup
An unofficial (SourceMod) *SourcePawn* toolchain multiplexer,
largely inspired by *Rustup* for the *Rust* programming language.

This project started off as being for personal use only,
but I figured that it might be useful for someone else.

## Overview
A *SourcePawn toolchain* includes everything provided in a SourceMod distribution, in `addons/sourcemod/scripting`,
specifically:
- the SourcePawn compiler executable, `spcomp` or `spcomp64`, and
- the directory with `.inc` files required for interacting with SourceMod, `include`.

Basically, Rookup allows for managing multiple SourcePawn toolchains to exist on one machine and be easily available
when needed.
There are two separate executables used that accomplish this:
- the `rookup` CLI, which allows for inspection of installed toolchains and installation of new ones, and
- the `rookup-spcomp` proxy executable, which executes the appropriate `spcomp` executable based on configuration and
environment variables.

In addition to this, a configuration file in the *TOML* format is used to keep track of various settings for the
behavior of the CLI and the proxy executable,
but it can be modified with the `rookup` CLI.

## Setup and usage
Build with `cargo build --release`
or download one of the [releases on GitHub](https://github.com/b0mbie/rookup/releases).
You should have two executables: `rookup` and `rookup-spcomp`.
Put them somewhere easily accessible,
like in one of the directories in the `PATH` environment variable for your profile.

### Configuration
Rookup uses a per-profile configuration.
On first usage when configuration is needed (like when [installing a toolchain](#installing-a-toolchain)),
`rookup` will create a default configuration in the profile's configuration directory.
More specifically:
- in the directory specified by the `ROOKUP_CONFIG_HOME` environment variable,
- `$XDG_CONFIG_HOME/rookup` on Linux, or
- `C:\Users\<user>\AppData\Roaming\rookup` on Windows.

### Installing a toolchain
To install the latest stable toolchain, run one of:
```
rookup update
rookup update stable
```

To install the absolute latest toolchain, run:
```
rookup update latest
```

To install a toolchain of a specific version, run:
```
rookup update :<version prefix>
```
For example, these invokations will yield the same result if done at a point in time where `1.12.0.7207` is the latest
version of `1.12`:
```
rookup install :1.12
rookup install :1.12.0
rookup install :1.12.0.7207
```

Any toolchains that are downloaded are put into the profile's cache directory.
More specifically:
- in the directory specified by the `ROOKUP_TOOLCHAIN_HOME` environment variable,
- `$XDG_CACHE_HOME/rookup/toolchains` on Linux, or
- `C:\Users\<user>\AppData\Local\rookup\toolchains` on Windows.

### Selecting compiler versions
Rookup manages versions with the concept of *version selectors*.
A *version selector* is either a more-human-friendly *alias* like `stable` or `latest`
(which both have special meaning in `rookup install`),
or a version like `:1.12` or `:1.11.0.6970`.
*Aliases* are resolved to versions, and are stored in the configuration file
(see [Configuration](#configuration)).

An alias can be queried with:
```
rookup alias <alias>
```
... and set with:
```
rookup alias <alias> <version>
```

When invoking `rookup-spcomp`,
it will select an installed version specified by either
the `ROOKUP_TOOLCHAIN` environment variable, or
the configuration file.
The default version selector can be queried with:
```
rookup default
```
... and set with:
```
rookup default <version selector>
```

### Deleting unused toolchains
Rookup will consider any version that isn't specified in the configuration as "unused", which can be queried with:
```
rookup list-unused
```
... and deleted with:
```
rookup purge
```

### Using custom toolchains
Rookup supports using custom toolchains which are never considered as "unused".
More specifically:
- in the directory specified by the `ROOKUP_CUSTOM_TOOLCHAIN_HOME` environment variable,
- `$XDG_DATA_HOME/rookup/toolchains` on Linux, or
- `C:\Users\<user>\AppData\Roaming\rookup\toolchains` on Windows.
