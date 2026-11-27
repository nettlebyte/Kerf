param([string]$Wd)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class U32 {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hWnd, uint msg, UIntPtr wParam, IntPtr lParam);
    public const uint LEFTDOWN = 0x0002;
    public const uint LEFTUP = 0x0004;
    public const uint WM_KEYDOWN = 0x0100;
    public const uint WM_KEYUP = 0x0101;
}
"@
[U32]::SetProcessDPIAware() | Out-Null

function Shot([string]$path) {
    $b = New-Object System.Drawing.Bitmap 1920, 1080
    $g = [System.Drawing.Graphics]::FromImage($b)
    $g.CopyFromScreen(0, 0, 0, 0, $b.Size)
    $g.Dispose()
    $b.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $b.Dispose()
}

Get-Process -Name Kerf -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 400
Start-Process -FilePath (Join-Path $Wd 'target\release\Kerf.exe') -WorkingDirectory $Wd
Start-Sleep -Milliseconds 900

Start-Process -FilePath (Join-Path $Wd 'target\release\Kerf.exe') -ArgumentList '--capture' -WorkingDirectory $Wd
Start-Sleep -Milliseconds 1100

[U32]::SetCursorPos(620, 320) | Out-Null
Start-Sleep -Milliseconds 200
[U32]::mouse_event([U32]::LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
for ($i = 1; $i -le 30; $i++) {
    [U32]::SetCursorPos((620 + $i * 14), (320 + $i * 9)) | Out-Null
    Start-Sleep -Milliseconds 12
}
[U32]::mouse_event([U32]::LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 500

$ov = [U32]::FindWindowW('KerfOverlay', $null)
"overlay=$ov"
[U32]::PostMessageW($ov, 0x8001, [UIntPtr]1, [IntPtr]0) | Out-Null
Start-Sleep -Milliseconds 200
[U32]::SetCursorPos(660, 360) | Out-Null
Start-Sleep -Milliseconds 150
[U32]::mouse_event([U32]::LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
for ($i = 1; $i -le 24; $i++) {
    [U32]::SetCursorPos((660 + $i * 15), (360 + $i * 8)) | Out-Null
    Start-Sleep -Milliseconds 10
}
[U32]::mouse_event([U32]::LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 300
Shot (Join-Path $Wd 'tests\e2e\shot_annot.png')

[U32]::PostMessageW($ov, [U32]::WM_KEYDOWN, [UIntPtr]13, [IntPtr]0) | Out-Null
[U32]::PostMessageW($ov, [U32]::WM_KEYUP, [UIntPtr]13, [IntPtr]0) | Out-Null
Start-Sleep -Milliseconds 700
$img = [Windows.Forms.Clipboard]::GetImage()
if ($null -eq $img) { "CLIP_EMPTY" } else {
    "annot_clip=$($img.Width)x$($img.Height)"
    $img.Save((Join-Path $Wd 'tests\e2e\clip_annot.png'), [System.Drawing.Imaging.ImageFormat]::Png)
    $img.Dispose()
}

Start-Process -FilePath (Join-Path $Wd 'target\release\Kerf.exe') -ArgumentList '--settings' -WorkingDirectory $Wd
Start-Sleep -Milliseconds 1200
Shot (Join-Path $Wd 'tests\e2e\shot_settings.png')
$sh = [U32]::FindWindowW('KerfSettings', $null)
"settings=$sh"
[U32]::PostMessageW($sh, [U32]::WM_KEYDOWN, [UIntPtr]27, [IntPtr]0) | Out-Null
[U32]::PostMessageW($sh, [U32]::WM_KEYUP, [UIntPtr]27, [IntPtr]0) | Out-Null
Start-Sleep -Milliseconds 500
"settings_after_esc=$([U32]::FindWindowW('KerfSettings', $null))"

$alive = Get-Process -Name Kerf -ErrorAction SilentlyContinue
"kerf_alive=$($null -ne $alive)"
Get-Process -Name Kerf -ErrorAction SilentlyContinue | Stop-Process -Force
