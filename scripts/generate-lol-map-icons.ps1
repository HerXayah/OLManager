param(
    [string]$InputDir = "public/lol-map-icons",
    [string]$OutputDir = "public/lol-map-icons"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

function New-TintedIcon {
    param(
        [string]$SourcePath,
        [string]$DestPath,
        [System.Drawing.Color]$Tint
    )

    $bmp = [System.Drawing.Bitmap]::new($SourcePath)
    try {
        for ($y = 0; $y -lt $bmp.Height; $y++) {
            for ($x = 0; $x -lt $bmp.Width; $x++) {
                $px = $bmp.GetPixel($x, $y)
                if ($px.A -eq 0) {
                    continue
                }

                $lum = [int](0.299 * $px.R + 0.587 * $px.G + 0.114 * $px.B)
                $r = [int](($Tint.R * $lum) / 255)
                $g = [int](($Tint.G * $lum) / 255)
                $b = [int](($Tint.B * $lum) / 255)

                $out = [System.Drawing.Color]::FromArgb($px.A, $r, $g, $b)
                $bmp.SetPixel($x, $y, $out)
            }
        }

        $bmp.Save($DestPath, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $bmp.Dispose()
    }
}

if (-not (Test-Path -Path $InputDir)) {
    throw "Input directory not found: $InputDir"
}

if (-not (Test-Path -Path $OutputDir)) {
    New-Item -Path $OutputDir -ItemType Directory | Out-Null
}

$targets = @(
    "icon_ui_nexus_minimap_v2.png",
    "icon_ui_inhibitor_minimap_v2.png",
    "icon_ui_tower_minimap.png",
    "nexus.png",
    "inhibitor.png",
    "tower.png"
)

$blue = [System.Drawing.Color]::FromArgb(255, 69, 161, 255)
$red = [System.Drawing.Color]::FromArgb(255, 255, 82, 106)

foreach ($name in $targets) {
    $src = Join-Path $InputDir $name
    if (-not (Test-Path -Path $src)) {
        Write-Warning "Missing source icon, skipping: $src"
        continue
    }

    $base = [System.IO.Path]::GetFileNameWithoutExtension($name)
    $dstBlue = Join-Path $OutputDir ("{0}_blue.png" -f $base)
    $dstRed = Join-Path $OutputDir ("{0}_red.png" -f $base)

    New-TintedIcon -SourcePath $src -DestPath $dstBlue -Tint $blue
    New-TintedIcon -SourcePath $src -DestPath $dstRed -Tint $red

    Write-Host "Generated:" $dstBlue
    Write-Host "Generated:" $dstRed
}
