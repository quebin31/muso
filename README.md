<!--
 Copyright (C) 2020 kevin
 
 This file is part of muso.
 
 muso is free software: you can redistribute it and/or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.
 
 muso is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU General Public License for more details.
 
 You should have received a copy of the GNU General Public License
 along with muso.  If not, see <http://www.gnu.org/licenses/>.
-->


<p align="center">
    <br>
    <br>
    <image src="logo/muso.png" alt="muso"></image>
</p>

## About

**muso** is a CLI tool that helps you to keep your music folder sorted. It's
designed to be simple and fast, but also powerful and fully automated.
Currently **muso** supports MP3 and FLAC, but in the future it's planned to
support most codecs for audio.

## Concepts

### Format string
**muso** is all about renaming and moving files around, but how it'll decide
where the new file will reside, or which is going to be its name? Fortunately
you can tell **muso** how to rename your files with a *format string*. This
string will build the new name (path) using one or more of the following
placeholders:

- `{artist}`: The artist name (**album artist** from tags is preferred).
- `{album}`: The album name.
- `{title}`: Song title.
- `{track}`: Track number.
- `{ext}`: File extension (e.g. `mp3`, `flac`)

As an example, the default format that **muso** will use is the following.

```rs
"{artist}/{album}/{track} - {title}.{ext}"
```

A format string can be specified for *oneshot* mode using the `-f/--format`
option, or providing it in for each [library](#libraries) in the [config
file](share/config.toml).

### Libraries
We recently talked about libraries, these objects are used in the [config
file](share/config.toml) to provide **muso** settings while it's running in
*watcher* mode. For example, the default library provided in the [default config file](share/config.toml) is described as follows.

```toml
[libraries.default]
# Specified format that will be used for this library
format = '{artist}/{album}/{track} - {title}.{ext}'
# Folders that compose this library
folders = ['$HOME/Music']
# If enabled, the rename will be compatible with exFAT
exfat-compat = true

```

They are used to provide different options, to different folders. 

### Config file
**muso** will search for a config file in the following directories in order:
- `$XDG_CONFIG_DIR/muso/config.toml`
- `$HOME/.config/muso/config.toml`

It's also possible to indicate a custom path for config file with the
`-C/--config` option. Config file is primary used when running in *watcher*
mode, but it's also able to provide a default *format string* for certain
folders while running in *oneshot* mode. For example, in the [default config
file](share/config.toml) the default library specifies a format and a list of
folders, if you would run **muso** on `$HOME/Music` without specifying a
format, it'll try to grab it from the config file, if there isn't one that
correspond to the folder it'll fallback to the [default](#format-string).

## Usage
**muso** can be used in two modes: *oneshot* and *watcher*. Both of them have 
similar functionalities, but as the naming suggest they perform it differently.
Below we have the output of `muso --help`, which explains each option or flag available

```
USAGE:
    muso [FLAGS] [OPTIONS] [path]

FLAGS:
        --copy-service    Copy service file to systemd user config dir, nothing else
    -d, --dryrun          Don't create neither move anything
        --exfat-compat    Maintain names compatible with FAT32
    -h, --help            Prints help information
    -r, --recursive       Search for files recursively
    -V, --version         Prints version information
    -w, --watch           Watch libraries present in config

OPTIONS:
    -C, --config <config>    Custom config file location
    -f, --format <format>    Custom format string

ARGS:
    <path>    Working path to sort
```

Note about `--copy--service`, it'll only copy service file to systemd user config 
dir, and nothing else. **muso** won't do it's usual job of sorting and will fail
if other flags are provided.

### Oneshot
By the default, **muso** will run on the current working dir, but you can
provide your own path as a free argument. Config file is optional in this mode.

### Watcher
In this mode config file is required, and as it's described in section `[watch]` 
of the [default config file](share/config.toml), the watcher can be configured.

```toml
[watch]
every = 1 # second(s)
# Specifies which libraries will be seen by muso
libraries = [ 'default' ]
```

### Systemd service
It's recommended to invoke the *watcher* mode using the provided [service
file](share/muso.service) for `systemd`, this way you can run **muso**
automatically on boot. Service file should be run on user level (`systemd
--user`). The easiest way to copy the service file is running **muso** with
`--copy-service`, that's all.

## License

GNU General Public License v3.0 

See [LICENSE](LICENSE) to see the full text.