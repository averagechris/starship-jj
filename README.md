> Note: This is a private fork maintained by @averagechris. For the official upstream project, see:
> https://gitlab.com/lanastara_foss/starship-jj
# jj-starship

starship plugin for jj

## Features

- [x] show bookmarks in you current commits history and how many commits you are ahead of them.
  - [x] filter bookmarks by name.
  - [x] filter bookmarks by distance to current commit.
  - [x] limit number of bookmarks that will be printed.
  - [x] overwrite bookmark filter per workspace.
- [x] show current commit text.
- [x] show current commit state (Conflict, Divergent, Hidden).
- [x] show current commit metrics (changed files, insertions, deletions).
  - [x] define a custom template for how these changes should be presented.
- [x] print in colors.
- [x] customize settings via config file.
- [x] print a default config file.
- [x] print the path to the default config file path.
- [x] set custom config location via command line or environment args.

## Installation

### From Source

```bash
  cargo install starship-jj --locked
```

## Usage

1. Enable the plugin in you starship.toml

```toml
format="""
...
${custom.jj}\
...
"""

#...

[custom.jj]
command = "prompt"
format = "$output"
ignore_timeout = true
shell = ["starship-jj", "--ignore-working-copy", "starship"]
use_stdin = false
when = true
```

2. Configure what you want to see

starship-jj will load a configuration toml file either from the location provided via the --starship-config argument or from you OSs default config dir (Linus: "$XDG_CONFIG_DIR/starship-jj/starship-jj.toml" Windows: "%APPDATA%/starship-jj/starship-jj.toml").

If no config file exist starship-jj will use some sane default values.

You can see the default config location by using `starship-jj starship config path`.

You can also print the default configuration using `starship-jj starship config default`

The Repository also contains a starship-jj.toml file with all possible keys and documentation.
