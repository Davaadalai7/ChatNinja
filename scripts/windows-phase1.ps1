$ErrorActionPreference = 'Stop'
$appPath = (Resolve-Path 'src-tauri/target/debug/jutsu.exe').Path
# CI uses the production frontend assets, not a Vite dev server.
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class JutsuWindows {
    delegate bool EnumProc(IntPtr hwnd, IntPtr value);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback, IntPtr value);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint owner);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int count);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
    static IntPtr Find(uint pid) {
        IntPtr found=IntPtr.Zero;
        EnumWindows((hwnd, v) => { uint owner; GetWindowThreadProcessId(hwnd,out owner);
            var title=new StringBuilder(256); GetWindowText(hwnd,title,256);
            if(owner==pid && title.ToString()=="Jutsu" && IsWindowVisible(hwnd)) { found=hwnd; return false; }
            return true;
        },IntPtr.Zero); return found;
    }
    public static bool Visible(uint pid) { return Find(pid)!=IntPtr.Zero; }
    public static bool Close(uint pid) { var h=Find(pid); return h!=IntPtr.Zero && PostMessage(h,0x0010,IntPtr.Zero,IntPtr.Zero); }
}
'@
$results = [System.Collections.Generic.List[string]]::new()
function Assert-Visible($process) {
    for ($i=0; $i -lt 40; $i++) {
        if ($process.HasExited) { throw "App exited early: $($process.ExitCode)" }
        if ([JutsuWindows]::Visible([uint32]$process.Id)) { return }
        Start-Sleep -Milliseconds 250
    }
    throw 'Jutsu settings window did not appear.'
}
function Close-App($process) {
    if (-not [JutsuWindows]::Close([uint32]$process.Id)) { throw 'Could not close Jutsu window.' }
    if (-not $process.WaitForExit(8000)) { throw 'Jutsu remained alive after Quit/default window close.' }
    if ($process.ExitCode -ne 0) { throw "Nonzero exit: $($process.ExitCode)" }
}
$first = $null; $second = $null; $reopened = $null
try {
    $first = Start-Process $appPath -PassThru
    Assert-Visible $first
    $results.Add('PASS: settings window visible on first launch')
    $logPath = Join-Path $env:LOCALAPPDATA 'dev.star0x7f.jutsu/logs'
    $frontendReady = $false
    for ($i=0; $i -lt 40; $i++) {
        $logFiles = Get-ChildItem $logPath -Filter '*.log' -ErrorAction SilentlyContinue
        if ($logFiles -and ($logFiles | Select-String 'Settings frontend reached native bootstrap' -Quiet)) { $frontendReady = $true; break }
        Start-Sleep -Milliseconds 250
    }
    if (-not $frontendReady) { throw 'The settings frontend did not reach the permitted native bootstrap command.' }
    $results.Add('PASS: bundled frontend loaded and native IPC capability works')
    $second = Start-Process $appPath -PassThru
    if (-not $second.WaitForExit(8000)) { throw 'Second launch started an additional persistent process.' }
    Assert-Visible $first
    $results.Add('PASS: second instance exits and original remains visible')
    Close-App $first
    $results.Add('PASS: default window close exits process normally')
    $reopened = Start-Process $appPath -PassThru
    Assert-Visible $reopened
    Close-App $reopened
    $results.Add('PASS: reopen after full shutdown and quit again')
    $logPath = Join-Path $env:LOCALAPPDATA 'dev.star0x7f.jutsu/logs'
    if (-not (Get-ChildItem $logPath -Filter '*.log' -ErrorAction SilentlyContinue)) { throw 'File logs were not created.' }
    $results.Add('PASS: file log exists')
} finally {
    foreach ($p in @($first,$second,$reopened)) { if ($null -ne $p -and -not $p.HasExited) { Stop-Process -Id $p.Id -Force } }
    $results | Set-Content verification.txt
    $results | Write-Output
}
