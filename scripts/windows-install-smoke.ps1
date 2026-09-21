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
    public static bool OverlayVisible(uint processId) {
        bool found = false;
        EnumWindows((window, value) => {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            if (owner == processId && IsWindowVisible(window)) {
                var title = new StringBuilder(256);
                GetWindowText(window, title, title.Capacity);
                if (title.ToString() == "ChatNinja Overlay") found = true;
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@
    if (-not [ChatNinjaWindows]::OverlayVisible([uint32]$app.Id)) {
        throw 'The native process started, but the desktop overlay window is not visible.'
    }
    Write-Output 'PASS: silent install and native process startup on the Windows CI runner.'
    Write-Output 'PASS: overlay window visible after launch.'
} finally {
    if (-not $app.HasExited) {
        # Ask the dashboard to close, exercising normal app shutdown first.
        $null = $app.CloseMainWindow()
        if (-not $app.WaitForExit(10000)) {
            Stop-Process -Id $app.Id -Force
            throw 'ChatNinja did not exit after the dashboard was closed.'
        }
    }
}

$reopened = Start-Process -FilePath $appPath -PassThru
try {
    Start-Sleep -Seconds 10
    $reopened.Refresh()
    if ($reopened.HasExited) { throw "ChatNinja exited after reopening: $($reopened.ExitCode)." }
    if (-not [ChatNinjaWindows]::OverlayVisible([uint32]$reopened.Id)) {
        throw 'Overlay window was not restored after reopening ChatNinja.'
    }
    Write-Output 'PASS: relaunch restores a visible overlay.'
} finally {
    if (-not $reopened.HasExited) {
        $null = $reopened.CloseMainWindow()
        if (-not $reopened.WaitForExit(10000)) {
            Stop-Process -Id $reopened.Id -Force
            throw 'Reopened ChatNinja did not shut down normally.'
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
