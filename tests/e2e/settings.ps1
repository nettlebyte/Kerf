param([string]$Wd)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class S32 {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hWnd, uint msg, UIntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title);
    public const uint WM_KEYDOWN = 0x0100;
    public const uint WM_KEYUP = 0x0101;
}
"@
[S32]::SetProcessDPIAware() | Out-Null

function Shot([string]$path, [int]$w, [int]$h) {
    $b = New-Object System.Drawing.Bitmap $w, $h
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
Start-Process -FilePath (Join-Path $Wd 'target\release\Kerf.exe') -ArgumentList '--settings' -WorkingDirectory $Wd
Start-Sleep -Milliseconds 1200

$s = [S32]::FindWindowW('KerfSettings', $null)
"settings_hwnd=$s"
if ($s -ne [IntPtr]::Zero) {
    [S32]::SetForegroundWindow($s) | Out-Null
    Start-Sleep -Milliseconds 500
    [S32]::SetCursorPos(700, 460) | Out-Null
    Start-Sleep -Milliseconds 400
}
Shot (Join-Path $Wd 'tests\e2e\live_settings.png') 1920 1080

if ($s -ne [IntPtr]::Zero) {
    [S32]::PostMessageW($s, [S32]::WM_KEYDOWN, [UIntPtr]27, [IntPtr]0) | Out-Null
    [S32]::PostMessageW($s, [S32]::WM_KEYUP, [UIntPtr]27, [IntPtr]0) | Out-Null
    Start-Sleep -Milliseconds 700
    "window_alive=$([S32]::IsWindow($s))"
}
$alive = Get-Process -Name Kerf -ErrorAction SilentlyContinue
"kerf_alive=$($null -ne $alive)"
Get-Process -Name Kerf -ErrorAction SilentlyContinue | Stop-Process -Force
