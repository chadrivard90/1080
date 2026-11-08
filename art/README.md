# Custom artwork

Put image files here to replace the built-in vector art: backgrounds, emblems, dial faces and
cars. Any slot without a file keeps the built-in art. Prompts for generating each one with an
AI image tool are in [docs/ai-art-prompts.md](../docs/ai-art-prompts.md).

- **Name:** exactly the slot name, `.png` or `.jpg` (e.g. `emblem-f1.png`, `bg-wrc.jpg`).
- **Transparency:** slots marked *transparent* below must be PNG with a transparent background.
- **When it loads:** at app start. Restart Race Dash after adding files. The title screen footer
  shows "N custom images" when files were picked up.

## Preparing raw AI images (recommended)

Drop the files straight from the AI tool into `art/raw/` (named after the slot, any size, JPG
is fine), then run:

```
powershell -ExecutionPolicy Bypass -File tools\prep-art.ps1
```

It resizes and crops everything, removes black backgrounds from the slots that need
transparency, and writes the finished files into `art/`. `art/raw/` stays on your PC: it isn't
committed or shipped to the Pi.

## Two ways to get them onto the Pi

1. **Through GitHub (recommended):** put the files in this `art` folder on the PC, commit and push.
   The next build includes them, and **Update Race Dash** installs them.
2. **Straight onto the Pi (USB):** copy the files into `~/race-dash/art/` and restart the app.
   Updates never delete files in this folder.

## Slots

| File | Size (any similar ratio works) | Notes |
|---|---|---|
| `title-bg` | 1080 x 1920 (9:16) | Title screen background |
| `title-hero` | 1080 x 600 | Header art behind "RACE DASH" (keep the left side dark) |
| `emblem-f1` … `emblem-fh6` | 512 x 512 | Game emblems; cropped to a circle (`f1 nascar acevo ams2 dr2 wrc fm fh6`) |
| `bg-f1` … `bg-fh6` | 1080 x 1800 (3:5) | Dash background for each game, shown above the dock |
| `dr2-tach-face` | 1024 x 1024 | DiRT Rally tach face, blank (ticks, numbers and needle are drawn on top) |
| `dr2-needle` | 1024 x 1024, *transparent* | Tach needle pointing straight up, pivot at the exact centre |
| `dr2-gauge-face` | 1024 x 1024 | Face for the three small DiRT Rally gauges, blank |
| `car-gt` | 600 x 1200, *transparent* | Top-down GT car, nose up (AC EVO brake-bias view) |
| `car-drift` | 600 x 1200, *transparent* | Top-down car, nose up (Forza Horizon drift zone) |
| `car-rally` | 600 x 1200, *transparent* | Top-down rally car, nose up (WRC health matrix) |
| `tile-f1` … `tile-fh6` | 800 x 450 (16:9) | Optional art filling each title-screen button |
| `weather-clear`, `-cloud`, `-rain`, `-storm`, `-snow` | 512 x 512, *transparent* | Optional AMS2 weather desk icons |
| `icon-app`, `icon-update` | 256 x 256, *transparent* | Optional desktop icons for Race Dash and Update Race Dash |
