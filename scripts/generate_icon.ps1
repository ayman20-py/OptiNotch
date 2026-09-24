Add-Type -AssemblyName System.Drawing

function Draw-NotchIcon([int]$IconSize) {
    $bmp = New-Object System.Drawing.Bitmap($IconSize, $IconSize, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    $scale = [float]$IconSize / 32.0

    # 1. Top Bezel / Screen edge (Stroke #71717A, width 3, round cap)
    $penBezel = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 113, 113, 122), [float](3.0 * $scale))
    $penBezel.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $penBezel.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawLine($penBezel, [float](2.0 * $scale), [float](5.0 * $scale), [float](30.0 * $scale), [float](5.0 * $scale))
    $penBezel.Dispose()

    # 2. Notch Pill (x=3, y=11, w=26, h=15, rx=7.5, fill #E4E4E7, stroke #71717A width 2.5)
    $x = [float](3.0 * $scale)
    $y = [float](11.0 * $scale)
    $w = [float](26.0 * $scale)
    $h = [float](15.0 * $scale)
    $r = [float](7.5 * $scale)

    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc($x, $y, [float](2.0 * $r), [float](2.0 * $r), 180.0, 90.0)
    $path.AddArc([float]($x + $w - 2.0 * $r), $y, [float](2.0 * $r), [float](2.0 * $r), 270.0, 90.0)
    $path.AddArc([float]($x + $w - 2.0 * $r), [float]($y + $h - 2.0 * $r), [float](2.0 * $r), [float](2.0 * $r), 0.0, 90.0)
    $path.AddArc($x, [float]($y + $h - 2.0 * $r), [float](2.0 * $r), [float](2.0 * $r), 90.0, 90.0)
    $path.CloseFigure()

    $brushPill = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 228, 228, 231))
    $g.FillPath($brushPill, $path)
    $brushPill.Dispose()

    $penPill = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 113, 113, 122), [float](2.5 * $scale))
    $g.DrawPath($penPill, $path)
    $penPill.Dispose()
    $path.Dispose()

    # 3. Amber Status Dot (cx=16, cy=18.5, r=3, fill #F59E0B)
    $dotR = [float](3.0 * $scale)
    $dotCx = [float](16.0 * $scale)
    $dotCy = [float](18.5 * $scale)
    $brushDot = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 245, 158, 11))
    $g.FillEllipse($brushDot, [float]($dotCx - $dotR), [float]($dotCy - $dotR), [float](2.0 * $dotR), [float](2.0 * $dotR))
    $brushDot.Dispose()

    $g.Dispose()
    return $bmp
}

$WorkspaceRoot = Split-Path -Parent $PSScriptRoot
$AssetsDir = Join-Path $WorkspaceRoot "assets"
$IcoPath = Join-Path $AssetsDir "icon.ico"

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$pngEntries = @()

foreach ($sz in $sizes) {
    $bmp = Draw-NotchIcon $sz
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $ms.ToArray()
    $pngEntries += ,@($sz, $bytes)
    $ms.Dispose()
    $bmp.Dispose()
}

$fs = [System.IO.File]::Create($IcoPath)
$bw = New-Object System.IO.BinaryWriter($fs)

# ICONDIR Header: Reserved(2), Type(2, 1=Icon), Count(2)
$bw.Write([uint16]0)
$bw.Write([uint16]1)
$bw.Write([uint16]$pngEntries.Count)

$offset = 6 + ($pngEntries.Count * 16)

# ICONDIRENTRY list
foreach ($entry in $pngEntries) {
    $sz = $entry[0]
    $data = $entry[1]
    $bSize = if ($sz -ge 256) { [byte]0 } else { [byte]$sz }
    
    $bw.Write([byte]$bSize)         # Width (0 for 256)
    $bw.Write([byte]$bSize)         # Height (0 for 256)
    $bw.Write([byte]0)              # Color count
    $bw.Write([byte]0)              # Reserved
    $bw.Write([uint16]1)            # Color planes
    $bw.Write([uint16]32)           # Bits per pixel
    $bw.Write([uint32]$data.Length) # Image size in bytes
    $bw.Write([uint32]$offset)      # File offset

    $offset += $data.Length
}

# Image Data (PNG blocks)
foreach ($entry in $pngEntries) {
    $data = $entry[1]
    $bw.Write($data)
}

$bw.Flush()
$bw.Close()
$fs.Close()

Write-Host "Generated multi-resolution icon at: $IcoPath" -ForegroundColor Green
