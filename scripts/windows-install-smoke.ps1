$ErrorActionPreference = 'Stop'

# Run only on the disposable Windows CI runner. This is a packaging smoke test,
# not a replacement for real Windows 10/11, game, OBS or OAuth testing.
$installer = Get-ChildItem 'src-tauri/target/release/bundle/nsis/*.exe' |
    Select-Object -First 1
if (-not $installer) { throw 'NSIS installer was not produced.' }

$install = Start-Process -FilePath $installer.FullName -ArgumentList '/S' -PassThru -Wait
if ($install.ExitCode -ne 0) { throw "Installer failed with exit code $($install.ExitCode)." }

$installDir = Join-Path $env:LOCALAPPDATA 'ChatNinja'
$appPath = Join-Path $installDir 'chatninja.exe'
if (-not (Test-Path $appPath)) { throw 'Installed ChatNinja executable was not found.' }

$app = Start-Process -FilePath $appPath -PassThru
try {
    Start-Sleep -Seconds 10
    $app.Refresh()
    if ($app.HasExited) { throw "ChatNinja exited during startup: $($app.ExitCode)." }
    # A running process alone missed the original overlay regression. Verify that
    # the actual overlay window has been created and is visible after startup.
    Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class ChatNinjaWindows {
    public delegate bool EnumProc(IntPtr window, IntPtr value);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback, IntPtr value);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr window, StringBuilder title, int length);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window, out Rect bounds);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr window, uint message, IntPtr w, IntPtr l);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    static IntPtr FindOverlay(uint processId) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((window, value) => {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            if (owner == processId && IsWindowVisible(window)) {
                var title = new StringBuilder(256);
                GetWindowText(window, title, title.Capacity);
                if (title.ToString() == "ChatNinja Overlay") {
                    found = window;
                    return false;
                }
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static bool OverlayVisible(uint processId) { return FindOverlay(processId) != IntPtr.Zero; }
    public static bool CloseDashboard(uint processId) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((window, value) => {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            if (owner == processId) {
                var title = new StringBuilder(256);
                GetWindowText(window, title, title.Capacity);
                if (title.ToString() == "ChatNinja") { found = window; return false; }
            }
            return true;
        }, IntPtr.Zero);
        return found != IntPtr.Zero && PostMessage(found, 0x0010, IntPtr.Zero, IntPtr.Zero);
    }
    public static string WindowTitles(uint processId) {
        var result = new StringBuilder();
        EnumWindows((window, value) => {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            if (owner == processId) {
                var title = new StringBuilder(256);
                GetWindowText(window, title, title.Capacity);
                result.Append("[").Append(title).Append(" visible=").Append(IsWindowVisible(window)).Append("]");
            }
            return true;
        }, IntPtr.Zero);
        return result.ToString();
    }
    public static bool MoveResizeOverlay(uint processId, int x, int y, int width, int height) {
        var window = FindOverlay(processId);
        return window != IntPtr.Zero && SetWindowPos(window, IntPtr.Zero, x, y, width, height, 0x0014);
    }
    public static bool OverlayHasBounds(uint processId, int x, int y, int width, int height) {
        var window = FindOverlay(processId);
        if (window == IntPtr.Zero || !GetWindowRect(window, out Rect rect)) return false;
        return Math.Abs(rect.Left - x) <= 12 && Math.Abs(rect.Top - y) <= 12 &&
               Math.Abs(rect.Right - rect.Left - width) <= 12 &&
               Math.Abs(rect.Bottom - rect.Top - height) <= 12;
    }
}
'@
    if (-not [ChatNinjaWindows]::OverlayVisible([uint32]$app.Id)) {
        throw "The native process started, but the desktop overlay window is not visible. Windows: $([ChatNinjaWindows]::WindowTitles([uint32]$app.Id))"
    }
    Write-Output 'PASS: silent install and native process startup on the Windows CI runner.'
    Write-Output 'PASS: overlay window visible after launch.'
    if (-not [ChatNinjaWindows]::MoveResizeOverlay([uint32]$app.Id, 120, 140, 480, 320)) {
        throw 'Could not resize or move overlay on the Windows CI runner.'
    }
    Start-Sleep -Seconds 2 # Allow window events to update dashboard storage.
} finally {
    if (-not $app.HasExited) {
        # Process.CloseMainWindow can select the overlay in a two-window app.
        $closeSent = [ChatNinjaWindows]::CloseDashboard([uint32]$app.Id)
        if (-not $closeSent) { Write-Warning "Could not send WM_CLOSE to dashboard. Windows: $([ChatNinjaWindows]::WindowTitles([uint32]$app.Id))" }
        if (-not $app.WaitForExit(10000)) {
            Stop-Process -Id $app.Id -Force
            Write-Warning 'ChatNinja did not exit after dashboard WM_CLOSE.'
        }
    }
}

$reopened = Start-Process -FilePath $appPath -PassThru
try {
    Start-Sleep -Seconds 10
    $reopened.Refresh()
    if ($reopened.HasExited) { throw "ChatNinja exited after reopening: $($reopened.ExitCode)." }
    if (-not [ChatNinjaWindows]::OverlayVisible([uint32]$reopened.Id)) {
        throw "Overlay window was not restored after reopening ChatNinja. Windows: $([ChatNinjaWindows]::WindowTitles([uint32]$reopened.Id))"
    }
    if (-not [ChatNinjaWindows]::OverlayHasBounds([uint32]$reopened.Id, 120, 140, 480, 320)) {
        throw 'Overlay position or size was lost across application restart.'
    }
    Write-Output 'PASS: relaunch restores visible overlay and saved position/size.'
} finally {
    if (-not $reopened.HasExited) {
        $closeSent = [ChatNinjaWindows]::CloseDashboard([uint32]$reopened.Id)
        if (-not $closeSent) { Write-Warning "Could not send WM_CLOSE to reopened dashboard. Windows: $([ChatNinjaWindows]::WindowTitles([uint32]$reopened.Id))" }
        if (-not $reopened.WaitForExit(10000)) {
            Stop-Process -Id $reopened.Id -Force
            Write-Warning 'Reopened ChatNinja did not shut down after dashboard WM_CLOSE.'
        }
    }
}

$uninstaller = Join-Path $installDir 'uninstall.exe'
if (-not (Test-Path $uninstaller)) { throw 'Uninstaller was not found.' }
$uninstall = Start-Process -FilePath $uninstaller -ArgumentList '/S' -PassThru -Wait
if ($uninstall.ExitCode -ne 0) { throw "Uninstaller failed: $($uninstall.ExitCode)." }
for ($attempt = 0; $attempt -lt 20 -and (Test-Path $appPath); $attempt++) {
    Start-Sleep -Seconds 1
}
if (Test-Path $appPath) { throw 'Uninstall left the installed executable behind.' }
Write-Output 'PASS: normal shutdown and silent uninstall.'

$hash = Get-FileHash -Path $installer.FullName -Algorithm SHA256
"$($hash.Hash.ToLower())  $($installer.Name)" |
    Set-Content -Path "$($installer.FullName).sha256" -Encoding ascii
