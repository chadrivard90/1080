# Race Dash

A sim racing telemetry dashboard for a **Raspberry Pi** with a **1080x1920 portrait touch screen**, written in **Rust + Slint**.

- **Title screen** (racing themed): pick a game, see the IP address to send telemetry to, turn demo mode on or off, exit.
- **One themed dash per game** with three pages, original artwork (	ools/gen-art.js) and per-game fonts (scripts/fetch-fonts.sh).
- Every game shows a **track map** with car positions on one of its pages.
- Pop-up events (fastest lap, penalties, position changes…) appear in the dock, never over the data.
- Two desktop icons: **Race Dash** (launch) and **Update Race Dash** (download the latest build from GitHub).

## Games

| Game | Dash theme | Pages | Telemetry source |
|---|---|---|---|
| **F1 25** | Halo MFD: carbon fibre, recessed MFD panels, neon RPM sweep | WHEEL, ENGINEER, STRATEGY | UDP 20777, format 2025 |
| **NASCAR 26** | Spotter & Strategy: white / racing blue / red broadcast banners | THE PACK, THE DRAFT, PIT BOX | **PC bridge** (iRacing-engine shared memory) |
| **Assetto Corsa EVO** | MoTeC C127: pure black, ghost-segment monospace digits | DASH, THE RIG, PIT BOX | **PC bridge** (shared memory) |
| **Automobilista 2** | Endurance Weather Board: navy blueprint HUD, crosshair panels | DASH, WEATHER, STRATEGY | UDP 5606, Project CARS 2 format |
| **DiRT Rally 2.0** | Group B Analog: grunge, printed tach + shaking needle, green LCD | TACH, SURVIVAL, PHYSICS | UDP 20777, extradata 3 |
| **EA SPORTS WRC** | Stage Navigator: chopped magenta panels, shaking handbrake alert | STAGE, SERVICE, SPLITS | UDP 20777, default "wrc" structure |
| **Forza Motorsport** | Turn 10 Telemetry: slate glass, gradient arcs, G trail, dyno | THE APEX, PHYSICS, THE DYNO | UDP 20440, "Car Dash" format |
| **Forza Horizon 6** | Festival Cruiser: neon gradient, RPM equalizer, drift zone | CRUISE, DRIFT, EXPLORER | UDP 20440 |

Every dash has a **dock** along the bottom: three page buttons, a centre **pill** that shows flags and pop-up events (so nothing covers the dash), and a red **MENU** button. While a flag is out, the whole screen edge also glows in the flag colour. Page heights scale with the screen, so nothing gets pushed off the top.
Design briefs for each theme are in [docs/game-themes.md](docs/game-themes.md).

### Track maps

No game sends a track outline, so Race Dash **draws the track from your own driving line**. The outline completes after your first lap; rally stages draw as you drive; Forza Horizon shows a trail around you. **Tap the map to rotate or mirror it** (remembered per game).

- Other cars appear on the map for **F1 25, AMS2, AC EVO and NASCAR**.
- **Forza** only sends your own car, so its maps show you only. Rally games have no other cars on the stage.

### What each game does *not* send

The dashes show what each game actually outputs. Where a brief asked for something the game doesn't send, the panel shows the closest available data:

- **DiRT Rally 2.0:** no water temp, oil pressure, fuel or tyre data. The analog dash shows brake temps and speed instead.
- **EA WRC (default packet):** no turbo boost or damage channels. The health matrix shows brake temps, suspension and wheel grip.
- **AMS2 UDP:** no "water on track" value or pit-menu plan. The dash shows rain density, wind, fuel, tyre wear and pit status.
- **NASCAR:** I couldn't confirm NASCAR 25/26's telemetry interface. The bridge reads the **iRacing SDK** shared memory, which iRacing-engine titles normally expose. If the bridge window never says "Connected to NASCAR", the game doesn't publish it.

## Layout rules (all dashes)

The screen sits to the right of the main monitor, so you see it out of the corner of your eye:

- **Colour carries the meaning.** Gaps, delta, fuel and temperatures change colour as they change state.
- **The flag bar is at the top of every tab** and turns solid yellow, blue or red for flags, cautions and the pit limiter.
- **Panels have fixed sizes.** Longer text is cut off with "…" instead of pushing neighbouring panels around.

---

## Setup

### 1. Put the code on GitHub (from this PC)

Install [Git for Windows](https://git-scm.com/download/win), create an empty **public** repo on GitHub (e.g. `race-dash`), then in this folder:

```bash
git init
```

```bash
git add .
```

```bash
git commit -m "Race Dash"
```

```bash
git branch -M main
```

```bash
git remote add origin https://github.com/<your-user>/race-dash.git
```

```bash
git push -u origin main
```

### 2. GitHub builds everything

Every push to `main` runs [.github/workflows/release.yml](.github/workflows/release.yml), which tests and builds:

- **`race-dash-aarch64.tar.gz`**: the Pi app (64-bit Raspberry Pi OS)
- **`race-dash-bridge.exe`**: the PC bridge for AC EVO and NASCAR

Both are published as the latest **Release**. Watch progress in the repo's **Actions** tab; a red X means a build failed, and the log says why.

### 3. First install on the Pi (USB)

1. From the repo's **Releases**, download **race-dash-aarch64.tar.gz** (not "Source code").
2. Copy it to a USB stick and plug it into the Pi.
3. In a terminal on the Pi:

```bash
mkdir -p ~/race-dash && tar -xzf /media/$USER/*/race-dash-aarch64.tar.gz -C ~/race-dash
```

```bash
bash ~/race-dash/scripts/setup.sh
```

This creates the **Race Dash** and **Update Race Dash** desktop icons. After this, **Update Race Dash** downloads new versions straight from GitHub.

### 4. Point each game at the Pi

Use the IP address shown on the Race Dash title screen. Each dash also shows these steps on screen until data arrives.

| Game | Settings |
|---|---|
| **F1 25** | Settings → Telemetry: UDP **On**, IP = Pi, Port **20777**, Send Rate 60 Hz, Format **2025** |
| **AMS2** | Options → System: Shared Memory **Project CARS 2**, UDP Protocol **Project CARS 2**, UDP Frequency **1**. AMS2 broadcasts on port 5606; the Pi must be on the same network. |
| **DiRT Rally 2.0** | Edit `Documents\My Games\DiRT Rally 2.0\hardwaresettings\hardware_settings_config.xml`: `<udp enabled="true" extradata="3" ip="PI_IP" port="20777" delay="1" />` |
| **EA WRC** | Edit `Documents\My Games\WRC\telemetry\config.json`: enable the **wrc** structure's packets and set `"ip"` to the Pi and `"port": 20777` |
| **Forza Motorsport** | Settings → Gameplay & HUD: Data Out **On**, IP = Pi, Port **20440**, Packet Format **Car Dash** |
| **Forza Horizon 6** | Settings → HUD and Gameplay: Data Out **On**, IP = Pi, Port **20440** (avoid 5200–5300) |
| **AC EVO / NASCAR** | Run **race-dash-bridge.exe** on the gaming PC (download it from Releases). It finds the running game and broadcasts to the Pi automatically. To send to one address instead, run `race-dash-bridge.exe PI_IP`. Allow it through Windows Firewall if asked. |

### Updating

Push changes to GitHub and wait for the Actions build to go green. Then double-click **Update Race Dash** on the Pi. Download a new `race-dash-bridge.exe` from Releases when the bridge changes.

---

## Demo mode

Tap **DEMO** on the title screen, then any game. Each dash runs simulated data (a race with a field of cars, a rally stage, or an open-world drive) so you can try every tab without the game.

## Settings page

Tap **SETTINGS** on the title screen:

- **Display:** panel see-through level (Solid / Glass / Clear) and demo mode.
- **Per game:** units, UDP port, shift-light point (% of max RPM, for games that don't send
  their own shift lights) and fuel margin (extra laps of fuel to keep in hand; used for
  "fuel to add" and for when F1's fuel delta turns red). Changes apply the next time you open
  that game's dash.
- **Lap log:** the all-time best per track and car for the selected game.

## Lap log

Every completed lap is appended to `~/race-dash-logs/laps.csv` (outside the install folder, so
updates never touch it). Columns: UTC time, game, track, car, session, lap, lap time, seconds,
valid, position, fuel used, personal best. The fastest valid lap per game + track + car is
shown on the dashes as **ALL-TIME**, with the last lap compared against it, and a purple
pop-up appears when you beat it. Rally games log a stage only if the game reports a finished
stage time.

## Settings file

`~/.config/race-dash/config.txt` (written automatically):

```
demo=0
glass=1              # panels: 0 solid, 1 glass, 2 clear
layout_f1=0          # last tab per game: f1, nascar, acevo, ams2, dr2, wrc, fm, fh6
map_f1=0             # map rotation 0-7 per game
port_fm=20440        # optional UDP port override per game
units_nascar=imperial  # optional: metric | imperial per game (NASCAR defaults to imperial)
shift_acevo=97       # shift lights full at this % of max RPM
fuelmargin_f1=0.5    # extra laps of fuel to keep in hand
```

## Development

- Run in a window at half size: `RACE_DASH_WINDOWED=1 SLINT_SCALE_FACTOR=0.5 cargo run`
- Tests (packet parsers, maps, dials): `cargo test` and, for the bridge, `cd bridge && cargo test`
- Logs from the desktop launcher: `~/.cache/race-dash.log`

### Project layout

```
ui/app.slint                  root window, page switching
ui/title.slint                title screen
ui/common.slint               buttons, bars, pop-up strip
ui/map.slint                  track map (shared by every game)
ui/f1/                        F1 25 theme, shell and seven layouts
ui/sim/                       shared data model + themeable widgets for the other games
ui/games/<game>.slint         each game's themed dash and layouts
src/games/f1_25/              F1 25 parser, view, demo
src/games/sim/                shared engine: state, view, dials, demo
src/games/{forza,dr2,wrc,ams2,bridge_rx}.rs   per-game parsers
src/games/map.rs              track outline recorder + renderer
bridge/                       Windows PC bridge (AC EVO + iRacing-engine shared memory)
scripts/                      setup / update / launch / desktop icons / build-from-source
docs/game-themes.md           design brief for every game's theme
.github/workflows/release.yml GitHub build -> Pi tarball + bridge exe release
```
