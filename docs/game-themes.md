# Game dash themes

Design brief for each game's dash. Each game gets its own theme plus several layouts behind the bottom nav bar. The concept below becomes that game's default layout. Vector art (Slint `Path`) and images are allowed in themes.

Status: **F1 25 built.** Others pending.

---

## 1. F1 25 - "The Halo MFD" ✅ (`ui/f1/layout_mfd.slint`, default tab)
Theme: F1 TV / Mercedes-Red Bull steering wheel screens. Dark grey, sharp neon purple / green / red.
- **Top:** massive gear inside a curved F1-style RPM LED sweep. Delta to best lap (green = faster, red = slower).
- **Middle:** hybrid management. ERS battery %, deploy mode, big DRS light that flashes green on zone entry.
- **Bottom:** four-tyre matrix, surface vs core temperature (spots "surface hot, core cold"), brake temperature next to each wheel.

## 2. Assetto Corsa EVO - "The MoTeC Engineer"
Theme: professional MoTeC data logger. Black, sharp white / red / blue text, no gradients.
- **Top:** lap time tower (current / last / best). Linear multi-colour shift bar across the full 1080 px.
- **Middle:** live track map with car position. TC and ABS activation lights that flash when the systems intervene.
- **Bottom:** suspension and tyre data. Tyre pressures colour-coded for the GT3 PSI window. Brake bias dial.

## 3. EA SPORTS WRC - "The Stage Navigator"
Theme: gritty, high contrast, like EA WRC's UI. Dirt textures, sharp angles, cyan / magenta.
- **Top:** stage progress bar 0-100 %. Large gear and a simple flashing shift light.
- **Middle:** analog-style turbo boost gauge and RPM dial. Prominent handbrake light.
- **Bottom:** vehicle health matrix with glowing icons for engine, radiator/water temp and suspension damage, for planning the next service park.

## 4. DiRT Rally 2.0 - "The Group B Analog"
Theme: 80s/90s analog rally car. Textured matte black, yellow / off-white dials, physical-looking needles.
- **Top:** massive central analog tachometer that vibrates slightly with engine output.
- **Middle:** survival gauges, analog water temp and oil pressure.
- **Bottom:** high-contrast digital stage timer (00:00.00) in a retro LCD font.

## 5. NASCAR 26 - "Spotter & Strategy Station"
Theme: Fox / NBC NASCAR broadcast. Bold blue, red and yellow blocks, heavy blocky type.
- **Top:** blind-spot radar. Huge left/right indicators that light yellow when a car is alongside.
- **Middle:** relative leaderboard, 3 cars ahead and 3 behind, gaps to 0.01 s (is the draft working?).
- **Bottom:** pit strategy. Fuel in laps remaining. Right-side tyre wear estimate (the RF takes the abuse).

## 6. Forza Motorsport - "The Apex Tracker"
Theme: Turn 10's UI. Sleek slate grey, smooth magenta / cyan gradients, rounded elements.
- **Top:** sweeping digital RPM arc. Live split comparison against rival / best lap.
- **Middle:** friction circle (G-meter), a 2D dot showing braking/cornering G.
- **Bottom:** tyre friction and heat, showing slipping vs gripping.

## 7. Forza Horizon 6 - "The Festival Cruiser"
Theme: music festival. Glassmorphism, neon pink / yellow, stylised fonts.
- **Top:** massive stylised digital speedometer filling the top third.
- **Middle:** drift and angle data, a live drift-angle visual (from yaw) plus lateral G.
- **Bottom:** cruise stats. Session max speed, elevation, compass heading (road-trip GPS feel).

## 8. Automobilista 2 - "Endurance Weather Board"
Theme: clean and structural, focused on environment and endurance variables.
- **Top:** gear and RPM flanked by live flag panels (yellow / blue / green).
- **Middle:** environment. Track surface temp, ambient temp, water-on-track %, for deciding when to pit for wets.
- **Bottom:** pit stop plan. Fuel to add and tyre compound selected, to check before entering the box.

---

## Telemetry to verify before building each game
Check these against each game's actual telemetry output when that game's turn comes:
- **NASCAR 26:** blind-spot radar needs other cars' positions (or a car-left/right flag) in the output.
- **DiRT Rally 2.0:** its UDP output has no oil pressure; water temp may also be missing. A fallback may be needed.
- **EA WRC:** check which damage channels are exposed; stage progress comes from stage distance.
- **Forza Horizon 6:** elevation from position Y and heading from yaw. Forza's "Data Out" format needs confirming for FH6.
- **AMS2:** water on track and pit-menu settings come from shared memory / the Project CARS 2 UDP format. Confirm which are sent over UDP.
- **Assetto Corsa EVO:** confirm the telemetry interface (track map coordinates, TC/ABS activity, pressures).
