# AI art prompts

Prompts for every swappable image in Race Dash. Generate with any AI image tool (ChatGPT /
GPT-image, Midjourney, Gemini, Firefly, Ideogram…), save with the file name shown, and put the
files in the [`art`](../art/README.md) folder.

## Rules that apply to every image

- **No text, numbers, logos, brand names or real game artwork.** The app draws all text itself.
  Real logos are trademarks, and generated lettering always comes out garbled.
- **Keep it dark and calm where data sits.** Backgrounds sit behind big white and coloured
  numbers, so ask for low contrast, a dark centre and a vignette.
- **Make a whole game's set in one session** (background + emblem + any instrument), so the
  style matches.
- **Transparent PNGs:** for slots marked *transparent*, ask the tool for a transparent
  background. If it can't (Midjourney can't), ask for a plain flat background, then remove it
  with a background-removal tool.
- **Aspect ratio:** Midjourney: add `--ar 9:16`, `--ar 3:5`, `--ar 1:1` or `--ar 1:2`. ChatGPT:
  say "portrait 9:16" or "square". The app scales and crops to fit, so close is fine.

- **Gemini (and other tools without transparency):** for *transparent* slots, ask for a
  **solid pure black (#000000) background** instead. `tools/prep-art.ps1` removes black
  backgrounds automatically (see [art/README.md](../art/README.md)).

Each section gives a full prompt you can paste as-is.

---

## Title screen

### `title-bg` (portrait 9:16)
> Dark motorsport garage at night, carbon fibre panels and blurred red and white light streaks,
> subtle chequered flag pattern fading into shadow, cinematic, moody, very dark overall with a
> deep vignette, low contrast so white text and coloured buttons stand out on top, portrait
> 9:16, no text, no logos

### `title-hero` (wide, about 1.8:1)
> Wide cinematic banner of an abstract race car streaking from right to left with long red and
> white motion-blur light trails, glowing tail lights, the left 55% of the image dark and nearly
> empty for title text, black background, high contrast, no text, no logos

---

## Game emblems (square 1:1, cropped to a circle)

Each emblem is shown inside a 140 px circle on a dark coloured button, so keep the subject
centred inside a round composition with a simple background.

### `emblem-f1`
> Circular badge emblem of a sleek modern open-wheel formula race car seen from the side with a
> halo safety bar, glossy red and black livery, dark carbon background, centred in a circle,
> bold vector-style illustration, rim light, no text, no logos

### `emblem-nascar`
> Circular badge emblem of an American stock car racing on a banked oval, side view, bold blue,
> red and yellow broadcast colours, metallic shine, crowd lights blurred behind, centred,
> vector-style illustration, no numbers, no text, no logos

### `emblem-acevo`
> Circular badge emblem of a GT3 endurance race car at speed, three-quarter front view, silver
> and red livery, night circuit lights reflecting on the bodywork, dark background, premium
> automotive illustration, centred, no text, no logos

### `emblem-ams2`
> Circular badge emblem of a waving black-and-white chequered flag over a Brazilian-style race
> circuit at dusk, green and yellow accent lights, rain droplets in the air, centred,
> illustration, no text, no logos

### `emblem-dr2`
> Circular badge emblem of a 1980s Group B rally car jumping over a crest on a gravel stage,
> orange and black livery, dust and stones flying, dramatic low sun, gritty illustration,
> centred, no text, no logos

### `emblem-wrc`
> Circular badge emblem of a modern rally car sliding sideways through a forest gravel corner,
> cyan and magenta light accents, gravel spray frozen in motion, dark moody background, sharp
> angular illustration, centred, no text, no logos

### `emblem-fm`
> Circular badge emblem of a stylised race circuit outline glowing with a magenta to cyan
> gradient, a single sleek sports car on the racing line, dark slate background, clean modern
> illustration, centred, no text, no logos

### `emblem-fh6`
> Circular badge emblem of a festival-style sunset with neon pink and yellow light, a sports car
> silhouette on an open coastal road, palm trees, vibrant synthwave colours, centred,
> illustration, no text, no logos

---

## Dash backgrounds (portrait 3:5, about 1080 x 1800)

These sit behind the gauges, so ask for **very dark, low-detail, with a vignette** and nothing
important in the middle.

### `bg-f1`
> Close-up abstract texture of a Formula 1 steering wheel's carbon fibre face, very dark charcoal
> #15151E tones, faint teal and red LED glow at the edges, shallow depth of field, subtle,
> minimal detail in the centre, dark vignette, portrait 3:5, no text, no logos

### `bg-nascar`
> Clean bright brushed-aluminium and white broadcast-studio background, soft blue and red light
> beams at the edges, subtle diagonal stripes, very low contrast, plenty of plain space in the
> middle, portrait 3:5, no text, no logos

### `bg-acevo`
> Pure black background with an extremely faint, dark grey technical data-logger grid and a
> barely visible circuit-map line, almost entirely black, minimal, portrait 3:5, no text, no
> logos

### `bg-ams2`
> Deep navy #0A192F aerospace HUD background, faint cyan blueprint grid lines, subtle radar
> sweep and weather-satellite cloud texture very low in contrast, dark vignette, portrait 3:5,
> no text, no logos

### `bg-dr2`
> Worn matte black 1980s rally car dashboard panel, scratched and dusty textured paint, faint mud
> splatter at the edges, very dark overall, flat lighting, low contrast, portrait 3:5, no text,
> no logos

### `bg-wrc`
> Gritty dark grey #2C2C2C rally texture, gravel dust and tyre marks, faint magenta and cyan
> angular light streaks at the edges, very dark centre, high-grain film look, portrait 3:5, no
> text, no logos

### `bg-fm`
> Sleek dark slate grey #2B2E33 background with soft frosted-glass panels and smooth magenta to
> cyan gradient light leaks at the corners, premium automotive UI mood, minimal, very low
> contrast centre, portrait 3:5, no text, no logos

### `bg-fh6`
> Vibrant music-festival background, neon pink to yellow to mint gradient sky, blurred festival
> lights, bokeh and confetti, palm silhouettes at the bottom edge, slightly darkened so white
> text stays readable, portrait 3:5, no text, no logos

---

## Instruments (square 1:1)

The app draws the tick marks, numbers and needle **on top**, so the faces must be **blank**.

### `dr2-tach-face`
The first attempt came out tilted (an oval, not a circle) with a strip of background down one
side. The app draws the ticks, numbers and needle at exact positions, so the face must be a
perfect, flat, centred circle.
> Perfectly flat, straight-on orthographic product photograph of a vintage 1980s rally car
> tachometer, camera exactly centred and square to the dial with zero tilt and zero
> perspective, so the gauge is a perfect circle. The gauge fills the entire square image edge to
> edge, centred. Outer ring: brushed dark gunmetal bezel about 7% of the radius thick with a
> thin chrome inner lip. Inside: a plain matte black dial face with subtle worn texture and a
> faint curved glass reflection in the upper-left quarter. Completely blank dial: no numbers,
> no tick marks, no redline, no needle, no hub, no text, no logos. Solid pure black (#000000)
> in the four corners outside the circle. Square 1:1.

### `dr2-needle` (*transparent*)
> Single analog gauge needle on a transparent background, viewed straight on, pointing straight
> up, glowing orange-red painted metal needle, its round black pivot hub exactly at the centre
> of the square image, needle tip reaching about three-quarters of the way to the top edge,
> short counterweight tail below the hub, nothing else in the image

### `dr2-gauge-face`
> Top-down straight-on vintage auxiliary gauge face, perfectly centred circle filling the whole
> square image, thin chrome bezel at the outer edge, plain dark cream-tinted dial with subtle
> aged texture, BLANK face with no numbers, no ticks, no needle, no text, black corners outside
> the circle

---

## Cars (portrait 1:2, *transparent*)

Top-down, **nose pointing up**, filling the frame. The WRC health zones are drawn over
`car-rally` at fixed positions: engine in the front third, wheels at the four corners.

### `car-gt`
> Top-down orthographic view of a GT3 race car, nose pointing straight up, silver bodywork with
> a blue centre stripe, rear wing visible, detailed but clean, car fills the frame, transparent
> background, no text, no logos, no numbers

### `car-drift`
> Top-down orthographic view of a modified sports car, nose pointing straight up, glossy hot
> pink paint with yellow stripes, neon underglow, car fills the frame, transparent background,
> no text, no logos, no numbers

### `car-rally`
> Top-down orthographic view of a modern rally car, nose pointing straight up, dark grey
> bodywork with magenta accents, roof vent, mud flaps, car fills the frame, transparent
> background, no text, no logos, no numbers

---

## Extra slots (optional)

### Title button art: `tile-f1` … `tile-fh6` (wide 16:9)
Fills each game button behind its emblem and name. The app darkens the bottom half, so keep the
subject in the upper part.

> **tile-f1:** Cinematic wide shot of a modern open-wheel formula car at night under floodlights, red and black livery, motion blur, sparks under the floor, subject in the upper half, dark lower half, 16:9, no text, no logos, no numbers
>
> **tile-nascar:** Cinematic wide shot of three stock cars racing side by side on a high-banked oval at dusk, bold blue, red and yellow liveries, grandstand lights blurred, subject in the upper half, 16:9, no text, no logos, no numbers
>
> **tile-acevo:** Cinematic wide shot of a GT3 car braking hard into a corner at a European circuit at golden hour, glowing brake discs, heat haze, subject in the upper half, 16:9, no text, no logos, no numbers
>
> **tile-ams2:** Cinematic wide shot of prototype endurance race cars on a wet Brazilian circuit at dusk, spray trails, headlights cutting through rain, subject in the upper half, 16:9, no text, no logos
>
> **tile-dr2:** Cinematic wide shot of a 1980s Group B rally car flying over a crest on a gravel forest stage, dust plume, low sun, subject in the upper half, 16:9, no text, no logos, no numbers
>
> **tile-wrc:** Cinematic wide shot of a modern rally car drifting through a snowy forest stage at night, headlights, snow spray, cyan and magenta light accents, subject in the upper half, 16:9, no text, no logos
>
> **tile-fm:** Cinematic wide shot of a sleek sports race car on a modern circuit at twilight, magenta and cyan lighting, clean reflections, subject in the upper half, 16:9, no text, no logos
>
> **tile-fh6:** Cinematic wide shot of a tuned sports car cruising a coastal festival road at sunset, neon pink and yellow sky, palm trees, festival lights, subject in the upper half, 16:9, no text, no logos

### AMS2 weather icons: `weather-clear`, `weather-cloud`, `weather-rain`, `weather-storm`, `weather-snow` (square, *transparent*)
Make all five in one session so they match.

> Flat glowing weather icon in a clean aerospace-HUD style, thin cyan and mint neon line-art with
> a soft glow, centred, on a solid pure black (#000000) background, square 1:1, no text.
> Subject: **[a bright sun with rays]** / **[a cloud]** / **[a cloud with rain drops]** /
> **[a storm cloud with a lightning bolt and heavy rain]** / **[a cloud with snowflakes]**

### Panel textures: `panel-f1` … `panel-fh6` (square 1:1)
A subtle material drawn at 25% strength inside each theme's panels and stat cells. Keep them
**dark, even and low-contrast** with no focal point (they are cropped to every panel shape).

> **panel-f1:** Seamless close-up of glossy black carbon-fibre weave with a faint teal sheen, even lighting, no focal point, square 1:1, no text
>
> **panel-nascar:** Seamless brushed aluminium plate with fine horizontal grain and a soft white sheen, even lighting, square 1:1, no text
>
> **panel-acevo:** Seamless matte black anodised metal with an ultra-faint hexagonal carbon pattern, almost black, square 1:1, no text
>
> **panel-ams2:** Seamless deep navy HUD glass with a faint cyan hexagon grid and scan lines, very dark, square 1:1, no text
>
> **panel-dr2:** Seamless worn matte black painted steel with light scratches and dust, very dark, square 1:1, no text
>
> **panel-wrc:** Seamless dark charcoal gravel-dust texture with faint magenta speckles, gritty, very dark, square 1:1, no text
>
> **panel-fm:** Seamless frosted dark glass with a soft magenta-to-cyan light leak in one corner, smooth, square 1:1, no text
>
> **panel-fh6:** Seamless dark purple velvet with tiny scattered neon pink and yellow confetti sparkles, square 1:1, no text

### Extra cars: `car-f1`, `car-stock` (portrait 1:2, *transparent*)
`car-f1` sits between the four tyre cards on the F1 Engineer page; `car-stock` lights up the
NASCAR spotter radar when a car is alongside.

> **car-f1:** Top-down orthographic view of a modern Formula 1 car, nose pointing straight up, red and black livery, halo visible, car fills the frame, on a solid pure black (#000000) background, no text, no logos, no numbers
>
> **car-stock:** Top-down orthographic view of an American stock car, nose pointing straight up, bold blue, red and yellow livery, car fills the frame, on a solid pure black (#000000) background, no text, no logos, no numbers

### Racing flags: `flag-yellow`, `flag-blue`, `flag-green`, `flag-white`, `flag-red`, `flag-black`, `flag-chequered` (square)
Shown as a gently waving corner banner on any dash while that flag is out, and as an icon in
the bottom bar. **Use a magenta background** (the black and white flags would vanish on black);
`prep-art.ps1` keys the magenta out. Make all seven in one session so they match.

> A single motorsport marshal's flag on a short pole, waving in the wind with soft cloth folds,
> glossy realistic fabric with subtle stitching, dramatic rim light, flag fills most of the
> square image, centred, on a solid flat pure magenta (#FF00FF) background with no shadow on the
> background, square 1:1, no text, no logos.
> Flag: **[solid yellow]** / **[solid blue]** / **[solid green]** / **[solid white]** /
> **[solid red]** / **[solid black]** / **[black and white chequered]**

### F1 tyre compounds: `tyre-soft`, `tyre-medium`, `tyre-hard`, `tyre-inter`, `tyre-wet` (square)
Shown on the F1 tyre cards and in the standings rows. **Magenta background** (tyres are black).

> Side view of a modern Formula 1 racing tyre, perfectly round and straight-on, glossy black
> rubber with a bold coloured sidewall ring, rim light, tyre fills the square image, centred, on a
> solid flat pure magenta (#FF00FF) background, square 1:1, no text, no logos, no lettering on
> the sidewall.
> Ring: **[red, slick tread]** (soft) / **[yellow, slick tread]** (medium) / **[white, slick
> tread]** (hard) / **[green, grooved intermediate tread]** (inter) / **[blue, deep grooved wet
> tread]** (wet)

### Personal-best medal: `pb-medal` (square, *transparent*)
Shown in the bottom bar and the corner when you set a new all-time best, and in the lap log.

> Glossy purple and gold racing medal shaped like a stopwatch with a laurel wreath around it,
> glowing purple rim light, premium 3D render, centred, on a solid pure black (#000000)
> background, square 1:1, no text, no numbers, no letters

### Bottom-bar textures: `dock-f1` … `dock-fh6` (very wide, about 11:1)
A thin strip behind each dash's bottom navigation bar. Keep it dark so the buttons stand out.

> **dock-f1:** Very wide thin strip of dark carbon fibre weave with a thin red light line along the top edge, dark, ultra-wide 11:1, no text
>
> **dock-nascar:** Very wide thin strip of navy blue brushed metal with a red and white racing stripe along the top edge, ultra-wide 11:1, no text
>
> **dock-acevo:** Very wide thin strip of matte black anodised aluminium with a hairline red edge, ultra-wide 11:1, no text
>
> **dock-ams2:** Very wide thin strip of deep navy HUD glass with faint cyan scan lines, ultra-wide 11:1, no text
>
> **dock-dr2:** Very wide thin strip of worn black painted steel with rivets at each end, ultra-wide 11:1, no text
>
> **dock-wrc:** Very wide thin strip of dark gravel-dusted metal with a magenta edge light, ultra-wide 11:1, no text
>
> **dock-fm:** Very wide thin strip of dark frosted glass with a magenta-to-cyan gradient glow along the top, ultra-wide 11:1, no text
>
> **dock-fh6:** Very wide thin strip of dark purple with soft neon pink and yellow festival light bokeh, ultra-wide 11:1, no text

### Logo mark: `logo` (square, *transparent*)
Shown beside SIDEPOD on the title screen, on the start-up splash, and (if you don't make a
separate `icon-app`) as the desktop icon. Image only: the app draws the wordmark itself.

> Bold emblem for a racing telemetry app called Sidepod: a stylised sleek race car sidepod air
> intake shape merged with a glowing red tachometer arc, chrome and carbon fibre, red and white
> accents, symmetrical badge composition, premium 3D render, centred, on a solid pure black
> (#000000) background, square 1:1, no text, no letters, no logos

### Gauge faces: `wrc-gauge-face`, `fm-gauge-face` (square)
Blank printed dial faces behind the WRC RPM gauge and the Forza Motorsport rev arc. The app
draws ticks, numbers and the arc on top, so keep the faces **blank** and dark.

> **wrc-gauge-face:** Top-down straight-on modern rally car tachometer face, perfectly centred circle filling the whole square image, dark charcoal carbon texture with a thin magenta inner ring and subtle cyan glow at the rim, BLANK face with no numbers, no ticks, no needle, no text, black corners outside the circle, square 1:1
>
> **fm-gauge-face:** Top-down straight-on premium sports car gauge face, perfectly centred circle filling the whole square image, dark slate glass with a soft magenta-to-cyan gradient ring at the rim, BLANK face with no numbers, no ticks, no needle, no text, black corners outside the circle, square 1:1

### AMS2 wet-track icons: `weather-puddle`, `weather-wetline` (square, *transparent*)
Shown in the AMS2 water-on-track card when it's wet. Make them in the same session as the
other weather icons so the style matches.

> Flat glowing weather icon in a clean aerospace-HUD style, thin cyan and mint neon line-art with
> a soft glow, centred, on a solid pure black (#000000) background, square 1:1, no text.
> Subject: **[a puddle of standing water with ripples on a road surface]** /
> **[a curved racing line on a road with water spray drying along it]**

### Desktop icons: `icon-app`, `icon-update` (square, *transparent*)
> **icon-app:** App icon of an analog racing tachometer with a red needle in the redline and a small chequered-flag stripe across the top, glossy rounded-square badge, dark carbon background inside the badge, bold and readable at small size, centred on a solid pure black (#000000) background, square 1:1, no text
>
> **icon-update:** App icon of two circular arrows (refresh symbol) in green and red around a small downward arrow, glossy rounded-square badge, dark carbon background inside the badge, bold and readable at small size, centred on a solid pure black (#000000) background, square 1:1, no text
