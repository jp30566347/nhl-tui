# nhl-tui

NHL scores, standings, schedule, and leaders in your terminal.

## Installation

### Homebrew (macOS)

```sh
brew install jp30566347/tap/nhl-tui
```

### Download binary

Grab the latest binary for your platform from [GitHub Releases](https://github.com/jp30566347/nhl-tui/releases).

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `nhl-tui-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `nhl-tui-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `nhl-tui-x86_64-unknown-linux-musl.tar.gz` |
| Linux (ARM64) | `nhl-tui-aarch64-unknown-linux-musl.tar.gz` |
| Windows | `nhl-tui-x86_64-pc-windows-msvc.zip` |

### Build from source

```sh
git clone https://github.com/jp30566347/nhl-tui.git
cd nhl-tui
cargo build --release
```

## Usage

```sh
nhl-tui                    # Default: scores tab
nhl-tui --team TOR         # Highlight your favorite team
nhl-tui --tab 2            # Open standings tab (1=Scores, 2=Standings, 3=Schedule, 4=Leaders)
```

## Keybindings

| Key | Action |
|-----|--------|
| `1` `2` `3` `4` | Switch tabs |
| `h` `l` / `←` `→` | Previous/next day (Scores, Schedule), or cycle the filter (Standings, Leaders) |
| `j` `k` / `↓` `↑` | Move the selection |
| `g` `G` / `Home` `End` | Jump to first/last row |
| `Enter` | Open game boxscore |
| `r` | Refresh now |
| `Esc` | Close boxscore, or quit |
| `q` / `Ctrl-C` | Quit |

## Data

Scores and stats refresh automatically every 30 seconds from the [NHL API](https://api-web.nhle.com),
in the background so the interface stays responsive while requests are in flight.

## License

MIT
