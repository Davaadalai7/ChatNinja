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
    Write-Output 'PASS: silent install and native process startup on the Windows CI runner.'
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
