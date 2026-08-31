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
nhl-tui --team TOR         # Highlight your favorite team, with a goal alert
nhl-tui --tab 2            # Open on a tab (1=Scores, 2=Standings, 3=Schedule,
                           #                4=Skaters, 5=Goalies)
```

## Keybindings

Press `?` in the app for this list.

| Key | Action |
|-----|--------|
| `1` - `5` | Jump to a tab |
| `Tab` / `Shift-Tab` | Cycle tabs |
| `j` `k` / `↓` `↑` | Move the selection |
| `Ctrl-D` / `Ctrl-U` | Half page down / up |
| `PgDn` / `PgUp` | Full page down / up |
| `g` `G` / `Home` `End` | First / last row |
| `h` `l` / `←` `→` | Previous/next day (Scores, Schedule), or cycle category |
| `H` / `L` | Jump back / forward one week |
| `t` | Back to today |
| `Enter` | Open the boxscore (Scores tab) |
| `r` | Refresh everything now |
| `?` | Toggle help |
| `Esc` | Close an overlay |
| `q` / `Ctrl-C` | Quit |

## Data

Data comes from the [NHL API](https://api-web.nhle.com) and is fetched in the background, so the
interface stays responsive while requests are in flight.

Scores refresh every 30 seconds. Standings, the schedule, and season leaders change about once a
day, so they refresh every 5 minutes instead of on every cycle; `r` forces all of them immediately.

## License

MIT
