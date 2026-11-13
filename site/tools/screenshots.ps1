<#
.SYNOPSIS
  Captures the app screenshots used by the website and the README.

.DESCRIPTION
  For every language in site/tools/demo/<lang>/ and both themes this builds the
  release app, starts it with that demo data (BUDGET_DATA_DIR) and captures its
  window. Results:
    site/src/assets/screenshots/<lang>-<light|dark>.png
    docs/screenshot.png   (copy of en-light, used by the README)

  The app is driven with two developer overrides: CASHFLOW_MONTH (opens a fixed
  month) and CASHFLOW_THEME (light or dark). The script needs a visible desktop:
  it brings each window to the front and copies pixels from the screen, so do not
  work in other windows while it runs (about two minutes).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File site/tools/screenshots.ps1
  powershell -ExecutionPolicy Bypass -File site/tools/screenshots.ps1 -Languages fi,en -Themes light
#>
param(
  [string[]]$Languages = @('en', 'fi', 'sv', 'de', 'fr', 'es'),
  [string[]]$Themes = @('light', 'dark'),
  [string]$Month = '2026-11',
  [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$TitleBarPx = 31  # height of the Windows title bar in the captured frame
# `-File` passes "fi,en" as one string, so accept both forms.
$Languages = @($Languages | ForEach-Object { $_ -split ',' })
$Themes = @($Themes | ForEach-Object { $_ -split ',' })
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$exe = Join-Path $root 'target\release\cashflow.exe'
$outDir = Join-Path $root 'site\src\assets\screenshots'

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class CfWin {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte k, byte s, int f, int e);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int hh, bool r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int s);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
'@
[CfWin]::SetProcessDPIAware() | Out-Null

if (-not $SkipBuild) {
  Write-Host 'Building release app...'
  Push-Location $root
  try { cargo build --release --locked; if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' } }
  finally { Pop-Location }
}
if (-not (Test-Path $exe)) { throw "Missing $exe" }
New-Item -ItemType Directory -Force $outDir | Out-Null

function Get-FrameRect([IntPtr]$h) {
  $r = New-Object CfWin+RECT
  [CfWin]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) | Out-Null  # DWMWA_EXTENDED_FRAME_BOUNDS
  return $r
}

function Capture([string]$lang, [string]$theme, [string]$target) {
  $demo = Join-Path $PSScriptRoot "demo\$lang"
  if (-not (Test-Path $demo)) { throw "No demo data for '$lang' in $demo" }
  # The app writes to its data directory, so work on a throw-away copy.
  $data = Join-Path ([IO.Path]::GetTempPath()) "cashflow-shot-$lang-$theme"
  if (Test-Path $data) { Remove-Item $data -Recurse -Force }
  Copy-Item $demo $data -Recurse

  $env:BUDGET_DATA_DIR = $data
  $env:CASHFLOW_MONTH = $Month
  $env:CASHFLOW_THEME = $theme
  $p = Start-Process $exe -PassThru
  try {
    $h = [IntPtr]::Zero
    for ($i = 0; $i -lt 30 -and $h -eq [IntPtr]::Zero; $i++) {
      Start-Sleep -Milliseconds 500
      $p.Refresh()
      $h = $p.MainWindowHandle
    }
    if ($h -eq [IntPtr]::Zero) { throw 'The app window did not appear' }
    Start-Sleep -Seconds 3

    $front = $false
    for ($try = 0; $try -lt 5 -and -not $front; $try++) {
      [CfWin]::MoveWindow($h, 100, 50, 1380, 900, $true) | Out-Null
      [CfWin]::keybd_event(0x12, 0, 0, 0); [CfWin]::keybd_event(0x12, 0, 2, 0)  # Alt unlocks SetForegroundWindow
      [CfWin]::ShowWindow($h, 9) | Out-Null
      [CfWin]::BringWindowToTop($h) | Out-Null
      [CfWin]::SetForegroundWindow($h) | Out-Null
      Start-Sleep -Seconds 2
      $front = ([CfWin]::GetForegroundWindow() -eq $h)
    }
    if (-not $front) { throw 'Could not bring the app window to the front' }

    [CfWin]::SetCursorPos(2, 2) | Out-Null  # keep the mouse off the window (no hover states)
    Start-Sleep -Milliseconds 500
    $r = Get-FrameRect $h
    $bmp = New-Object Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
    $g = [Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
    $g.Dispose()
    # Keep only the app content: drop the OS title bar and the 1 px window border.
    # The website draws its own generic window frame around the image.
    $cropped = $bmp.Clone((New-Object Drawing.Rectangle 1, $TitleBarPx, ($bmp.Width - 2), ($bmp.Height - $TitleBarPx - 1)), $bmp.PixelFormat)
    $cropped.Save($target, [Drawing.Imaging.ImageFormat]::Png)
    $cropped.Dispose()
    $bmp.Dispose()
    Write-Host "  $target"
  }
  finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
    Remove-Item $data -Recurse -Force -ErrorAction SilentlyContinue
  }
}

try {
  foreach ($lang in $Languages) {
    foreach ($theme in $Themes) {
      Capture $lang $theme (Join-Path $outDir "$lang-$theme.png")
    }
  }
}
finally {
  Remove-Item Env:\BUDGET_DATA_DIR, Env:\CASHFLOW_MONTH, Env:\CASHFLOW_THEME -ErrorAction SilentlyContinue
}

$readme = Join-Path $outDir 'en-light.png'
if (Test-Path $readme) { Copy-Item $readme (Join-Path $root 'docs\screenshot.png') -Force }
Write-Host 'Done.'
