# Prepares raw AI images in art/raw/ into the sizes and formats the app wants, written to art/.
#   powershell -ExecutionPolicy Bypass -File tools\prep-art.ps1
# Originals are kept in art/raw/ (not loaded by the app) so this can be re-run any time.
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$raw = Join-Path $root "art\raw"
$out = Join-Path $root "art"
Add-Type -AssemblyName System.Drawing
Add-Type -Path (Join-Path $PSScriptRoot "ArtTool.cs") -ReferencedAssemblies System.Drawing

function Load($name) {
    foreach ($ext in "png", "jpg", "jpeg") {
        $p = Join-Path $raw "$name.$ext"
        if (Test-Path $p) { return [System.Drawing.Bitmap]::FromFile($p) }
    }
    return $null
}
function SavePng($bmp, $name) { $bmp.Save((Join-Path $out "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png); Write-Host "  $name.png" }
function SaveJpg($bmp, $name) { [ArtTool]::SaveJpg($bmp, (Join-Path $out "$name.jpg"), 88); Write-Host "  $name.jpg" }

Write-Host "Backgrounds"
foreach ($g in "f1", "nascar", "acevo", "ams2", "dr2", "wrc", "fm", "fh6") {
    $src = Load "bg-$g"; if ($src) { SaveJpg ([ArtTool]::Cover($src, 1080, 1800)) "bg-$g"; $src.Dispose() }
}
$src = Load "title-bg"; if ($src) { SaveJpg ([ArtTool]::Cover($src, 1080, 1920)) "title-bg"; $src.Dispose() }
$src = Load "title-hero"; if ($src) { SaveJpg ([ArtTool]::Cover($src, 1080, 600)) "title-hero"; $src.Dispose() }

Write-Host "Panel textures"
foreach ($g in "f1", "nascar", "acevo", "ams2", "dr2", "wrc", "fm", "fh6") {
    $src = Load "panel-$g"; if ($src) { SaveJpg ([ArtTool]::Cover($src, 1024, 1024)) "panel-$g"; $src.Dispose() }
}

Write-Host "Title button art"
foreach ($g in "f1", "nascar", "acevo", "ams2", "dr2", "wrc", "fm", "fh6") {
    $src = Load "tile-$g"; if ($src) { SaveJpg ([ArtTool]::Cover($src, 800, 450)) "tile-$g"; $src.Dispose() }
}

Write-Host "Weather + desktop icons (background removed)"
foreach ($n in "weather-clear", "weather-cloud", "weather-rain", "weather-storm", "weather-snow", "icon-app", "icon-update") {
    $src = Load $n
    if ($src) {
        # Weather icons are glowing line-art: every dark pixel (even enclosed ones) becomes see-through.
        # Desktop icons keep their solid badge, so only the outer black is removed.
        $core = if ($n -like "weather-*") { 255 } else { 30 }
        $cut = [ArtTool]::RemoveBlackGlow($src, $core)
        $box = [ArtTool]::OpaqueBox($cut)
        $size = if ($n -like "icon-*") { 256 } else { 512 }
        SavePng ([ArtTool]::FitInto($cut, $box, $size, $size, 0.04)) $n
        $cut.Dispose(); $src.Dispose()
    }
}

Write-Host "Emblems (cropped tight to the badge)"
foreach ($g in "f1", "nascar", "acevo", "ams2", "dr2", "wrc", "fm", "fh6") {
    $src = Load "emblem-$g"
    if ($src) {
        $threshold = if ($g -eq "fh6") { 90 } else { 40 }
        $box = [ArtTool]::BrightBox($src, $threshold)
        SaveJpg ([ArtTool]::SquareAround($src, $box, 0.0, 512)) "emblem-$g"
        $src.Dispose()
    }
}

Write-Host "Gauge face"
$src = Load "dr2-gauge-face"
if ($src) { $box = [ArtTool]::BrightBox($src, 40); SaveJpg ([ArtTool]::SquareAround($src, $box, 0.0, 1024)) "dr2-gauge-face"; $src.Dispose() }
$src = Load "dr2-tach-face"
if ($src) { $box = [ArtTool]::BrightBox($src, 40); SaveJpg ([ArtTool]::SquareAround($src, $box, 0.0, 1024)) "dr2-tach-face"; $src.Dispose() }

Write-Host "Cars (background removed, fitted to 600 x 1200)"
foreach ($c in "car-gt", "car-drift", "car-rally", "car-f1", "car-stock") {
    $src = Load $c
    if ($src) {
        $cut = if ($c -eq "car-drift") { [ArtTool]::RemoveBlackGlow($src, 110) } else { [ArtTool]::RemoveBlack($src, 22, 70) }
        $box = [ArtTool]::OpaqueBox($cut)
        SavePng ([ArtTool]::FitInto($cut, $box, 600, 1200, 0.02)) $c
        $cut.Dispose(); $src.Dispose()
    }
}

Write-Host "Needle (background removed, pivot centred)"
$src = Load "dr2-needle"
if ($src) {
    $cut = [ArtTool]::RemoveBlack($src, 22, 70)
    $box = [ArtTool]::OpaqueBox($cut)
    $py = [ArtTool]::WidestRow($cut, $box)
    $px = [int]($box.X + $box.Width / 2)
    # Built-in needle: tip reaches 0.385 of the canvas above the pivot.
    SavePng ([ArtTool]::CentreNeedle($cut, $px, $py, $box.Top, 1024, 0.385)) "dr2-needle"
    Write-Host ("    pivot at {0},{1}  tip at y={2}  box {3}" -f $px, $py, $box.Top, $box)
    $cut.Dispose(); $src.Dispose()
}
Write-Host "Done."
