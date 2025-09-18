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

---

## Fork-specific: Config loading and .env behavior

This fork optimizes the hot path by avoiding heavy config merging and environment scanning on each prompt draw.

- No config crate merge in the hot path: the config file is parsed once via TOML. If no file is present, sensible defaults are used.
- Targeted env overrides: only specific `SJJ__*` variables are read (no full environment scan).
- `.env` is feature-gated and off by default to avoid I/O on every run.

### Supported environment overrides (prefix `SJJ__`)

- `SJJ__MODULE_SEPARATOR`: string separator between modules
- `SJJ__TIMEOUT`: integer milliseconds (e.g. `250`)
- `SJJ__RESET_COLOR`: boolean (`true/false/on/off/1/0`)
- `SJJ__BOOKMARKS__SEARCH_DEPTH`: unsigned integer
- `SJJ__BOOKMARKS__EXCLUDE`: comma-separated glob list (e.g. `"r/*,wip/*"`)

Examples:

```bash
SJJ__MODULE_SEPARATOR="|" starship-jj starship prompt
SJJ__BOOKMARKS__EXCLUDE="r/*,wip/*" starship-jj starship prompt
```

### Using `.env` (opt-in)

- Cargo feature `dotenv` enables reading `.env` from the current directory.
- Debug builds automatically load `.env` when the feature is enabled.
- Release builds require an explicit opt-in with `STARSHIP_JJ_USE_DOTENV=1`.

Cargo examples:

```bash
# Default (dotenv disabled): ignores .env
cargo run -- starship prompt

# Enable dotenv in debug: reads .env automatically
cargo run --features dotenv -- starship prompt

# Enable dotenv in release: requires opt-in
STARSHIP_JJ_USE_DOTENV=1 cargo run --release --features dotenv -- starship prompt
```

`.env` example:

```env
SJJ__MODULE_SEPARATOR="/"
```

Notes:
- `.env` is read from the current working directory.
- Avoid trailing spaces in `.env` values.

### Nix builds and dev shells

- Default package (dotenv off): `nix run .#starship-jj -- starship prompt`
- Dotenv-enabled package: `nix run .#starship-jj-dotenv -- starship prompt`
  - Since this is a release build, set `STARSHIP_JJ_USE_DOTENV=1` to load `.env`:
    ```bash
    STARSHIP_JJ_USE_DOTENV=1 nix run .#starship-jj-dotenv -- starship prompt
    ```
- Dev shells:
  - Default: `nix develop` (dotenv off) — use direct env overrides
  - Dotenv-ready: `nix develop .#dotenv` — pairs well with `cargo run --features dotenv`

Rationale: Starship invokes this binary frequently; disabling implicit `.env` and broad env scanning avoids unnecessary work at each prompt draw while keeping explicit overrides and dev ergonomics.

