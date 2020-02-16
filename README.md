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

## Usage
**muso** can be used in two modes: *oneshot* and *watcher*. The oneshot mode
can be invoked directly from the terminal, just like any other program. 

```
$ muso
```
**muso** will run on the current directory, and will try to read tags from supported files, then it'll move them according to the specified format. This format can be personalized with the option `-f/--format`.

```
$ muso --format="{artist}/{album}/{track} - {title}.{ext}"
```

The following variables can be used to define the format:
- `{artist}`: The artist name (**album artist** from tags is preferred).
- `{album}`: The album name.
- `{title}`: Song title.
- `{track}`: Track number.
- `{ext}`: File extension (e.g. `mp3`, `flac`)

If you want to sort all your files recursively you can use the `-r/--recursive` flag, and finally if you don't want to actually move your files, but rather see what **muso** would do, you can use the `-d/--dryrun` flag. It's also possible to use valid names for exFAT with the `--exfat-compat` flag.

Additionally, if you want to invoke **muso** in *watcher* mode you must use 
the `-w/--watch` flag. Running in *watcher* mode depends on a config file 
and ignore all the other flags (except `-d/--dryrun`). 

**muso** will search for a config file in the following directories in order:
- `$XDG_CONFIG_DIR/muso/config.toml`
- `$HOME/.config/muso/config.toml`

It's also possible to indicate a custom path for config file with the `-C/--config` option.

When **muso** is running on *watcher* mode, it'll work only on
files/subdirectories changed on the defined *libraries* in config file. A
*library* utilizes an specific format and it's defined on certain folders.
For example the `default` library on the [default config
file](share/config.toml) is defined as follows.

```toml
[libraries.default]
# Specified format that will be used for this library
format = '{artist}/{album}/{track} - {title}.{ext}'
# Folders that compose this library
folders = ['$HOME/Music']
# If enabled, the rename will be compatible with exFAT
exfat-compat = true

```

Different *libraries* can use different formats, making it really powerful and customizable. Finally, it's also possible to configure the *watcher* as it's described in the section `[watch]` in the [default config file](share/config.toml).

```toml
[watch]
every = 1 # second(s)
# Specifies which libraries will be seen by muso
libraries = [ 'default' ]
```

## Watcher using `systemd`
It's recommended to invoke the *watcher* mode using the provided [service
file](share/muso.service) for `systemd`, this way you can run **muso**
automatically on boot. Service file should be run on user level (`systemd
--user`).

## License

GNU General Public License v3.0 

See [LICENSE](LICENSE) to see the full text.