# Race Dash

A sim racing telemetry dashboard for a **Raspberry Pi** with a **1080x1920 portrait touch screen**, written in **Rust + Slint**.

- **Title screen** (racing themed): pick a game, see the IP address to send telemetry to, turn demo mode on or off, exit.
- **One dash per game**, styled for that game, with a **nav bar** at the bottom for switching layouts and a **HOME** button.
- Two desktop icons: **Race Dash** (launch) and **Update Race Dash** (pull from GitHub and rebuild).

Supported now: **F1 25**. Shown as "coming soon": NASCAR 26, Assetto Corsa EVO, Automobilista 2, DiRT Rally 2.0, EA SPORTS WRC, Forza Motorsport, Forza Horizon 6.

---

## F1 25 layouts

The screen sits to the right of the main monitor, so you only ever see it out of the corner of your eye. Every layout follows these rules:

- **Colour carries the meaning.** Peripheral vision picks up colour and movement well but can't read small text, so gaps, delta, fuel and temperatures all change colour as they change state.
- **Important info goes top-left**, the part of the screen closest to the main monitor.
- **Shift lights and the flag bar are always visible** at the top of every layout. The bar turns solid yellow, blue or red for flags, safety car, VSC, red flag and pit limiter.
- **Event pop-ups** appear briefly near the bottom (fastest lap, DRS enabled, penalties, overtakes, contact).

| Tab | Use it for | What's on it |
|---|---|---|
| **RACE** | Default, wheel-to-wheel | Huge gear, speed and RPM bar, position and lap, throttle/brake bars. **Gap ahead** (turns green inside 1.0 s, i.e. DRS range) and **gap behind** (turns red under 1.0 s). Live delta-to-best bar, last/best lap, DRS state, ERS %/mode, fuel +/- laps, 4-corner tyre temps/wear, brake bias/diff. |
| **TIMING** | Quali / hot laps | Large live delta. The whole panel tints green or red. Current lap time, S1/S2/S3 boxes (purple/green/yellow), last/PB/ideal lap, session best. Lap-invalid warning. |
| **CAR** | Engineer view | Top-down car with a tyre card at each corner: carcass temp (colour-coded), surface temp, pressure, brake temp, wear. Wing/floor/sidepod/diffuser/engine/gearbox damage. PU component wear, ERS deploy/harvest, fuel, G-force dot. |
| **TOWER** | Where is everyone | Broadcast-style timing tower for the whole field: team colour, interval and gap to leader (in a race) or best lap and gap to P1 (practice/quali), tyre compound and age, pit stops, fastest-lap marker. Your row is highlighted. |
| **STRAT** | Under SC, long straights | Weather now and forecast with rain %, pit window and rejoin position, tyre age/wear, fuel, penalties/warnings/track limits, lap history with sectors. |

The last layout you picked is remembered.

---

## Setup

### 1. Put the code on GitHub (from this PC)

Install [Git for Windows](https://git-scm.com/download/win), create an empty repo on GitHub (e.g. `race-dash`), then in this folder:

```bash
git init
```

```bash
git add .
```

```bash
git commit -m "Race Dash: title screen + F1 25 dash"
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

Make the repo **public**. The free ARM build machines and the Pi's no-login download both rely on it.

### 2. GitHub builds the Pi version automatically

Every push to `main` runs [.github/workflows/release.yml](.github/workflows/release.yml) on a GitHub ARM64 machine. It runs the tests, builds the app for 64-bit Raspberry Pi OS, and publishes **`race-dash-aarch64.tar.gz`** as the latest release. Watch it under the repo's **Actions** tab. The first run takes about 10–15 minutes; later runs are faster because of caching. A red X means the build failed, and the log shows why.

### 3. Install on the Pi (one time, no compiling)

Needs 64-bit Raspberry Pi OS with the desktop. In a terminal on the Pi (replace `<your-user>/race-dash`):

```bash
curl -fsSL https://raw.githubusercontent.com/<your-user>/race-dash/main/scripts/pi-install.sh | bash -s <your-user>/race-dash
```

This downloads the latest release into `~/race-dash` and creates the **Race Dash** and **Update Race Dash** desktop icons.

> Prefer to compile on the Pi instead? `git clone` the repo and run `bash scripts/install-from-source.sh`. The updater detects which kind of install you have.
### 4. Point F1 25 at the Pi

In F1 25: **Settings → Telemetry Settings**

| Setting | Value |
|---|---|
| UDP Telemetry | On |
| UDP Broadcast Mode | Off (or On, which also works) |
| UDP IP Address | the IP shown on the Race Dash title screen |
| UDP Port | 20777 |
| UDP Send Rate | 60 Hz |
| UDP Format | **2025** |

### Updating

Push changes to GitHub and wait for the Actions build to go green. Then double-click **Update Race Dash** on the Pi. It downloads the newest release, replaces the app only if it changed, and offers to relaunch.

---

## Demo mode

Tap **DEMO** on the title screen, then **F1 25**. The dash runs a simulated race at Silverstone, including flags, VSC, events, delta and tyre wear, so you can try every layout without the game running.

## Settings file

`~/.config/race-dash/config.txt`

```
demo=0
f1_layout=0
f1_port=20777
```

## Development

- Run in a window at half size: `RACE_DASH_WINDOWED=1 SLINT_SCALE_FACTOR=0.5 cargo run`
- Parser tests (packet offsets, delta tracker): `cargo test`
- Logs from the desktop launcher: `~/.cache/race-dash.log`

### Project layout

```
ui/app.slint                 root window, page switching
ui/title.slint               title screen
ui/common.slint              shared widgets (buttons, bars)
ui/f1/f1_state.slint         F1 data structs, F1 global, F1 theme colours
ui/f1/f1_dash.slint          shift lights, flag bar, nav bar, overlays
ui/f1/layout_*.slint         the five F1 layouts
src/main.rs                  app wiring
src/games/f1_25/packets.rs   F1 25 UDP parser (format 2025) + delta tracker
src/games/f1_25/view.rs      state -> UI data (formatting, colours)
src/games/f1_25/demo.rs      demo data generator
scripts/                     pi-install / update / launch / desktop icons / build-from-source
.github/workflows/release.yml  GitHub build -> race-dash-aarch64.tar.gz release
```

### Adding the next game

1. `src/games/<game>/`: a listener that parses that game's telemetry, plus a `view.rs`.
2. `ui/<game>/`: theme, layouts and a dash with the same nav bar pattern.
3. Enable its tile in `ui/title.slint`, add a page in `ui/app.slint`, and a match arm in `src/main.rs`.
