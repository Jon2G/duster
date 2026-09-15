# Install duster on Windows
# Usage:
#   irm https://raw.githubusercontent.com/Jon2G/duster/main/install.ps1 | iex
# Or:
#   .\install.ps1

$ErrorActionPreference = "Stop"

$Repo = "Jon2G/duster"
$AssetName = "duster-windows-x86_64.zip"
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\duster"
$Url = "https://github.com/$Repo/releases/latest/download/$AssetName"

Write-Host "Downloading duster for Windows x86_64..."
$tempZip = Join-Path $env:TEMP "duster-windows.zip"
$tempExtract = Join-Path $env:TEMP "duster-extract"

try {
    Invoke-WebRequest -Uri $Url -OutFile $tempZip -UseBasicParsing
} catch {
    Write-Error "Failed to download $Url. Ensure a Windows release exists for $Repo."
    exit 1
}

if (Test-Path $tempExtract) {
    Remove-Item -Recurse -Force $tempExtract
}
New-Item -ItemType Directory -Path $tempExtract | Out-Null
Expand-Archive -Path $tempZip -DestinationPath $tempExtract -Force

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$exe = Get-ChildItem -Path $tempExtract -Filter "duster.exe" -Recurse | Select-Object -First 1
if (-not $exe) {
    Write-Error "duster.exe not found in archive."
    exit 1
}
Copy-Item -Force $exe.FullName (Join-Path $InstallDir "duster.exe")

Remove-Item -Force $tempZip -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $tempExtract -ErrorAction SilentlyContinue

Write-Host "Installed to $InstallDir\duster.exe"

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not ($userPath -split ";" | Where-Object { $_ -ieq $InstallDir })) {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "Added $InstallDir to your user PATH."
    Write-Host "Restart your terminal, or run: `$env:Path += ';$InstallDir'"
}

Write-Host ""
Write-Host "duster installed successfully."
Write-Host "Run 'duster --help' to get started."
Write-Host "Tip: AppData scan is opt-in: duster scan --cache --include-sensitive"
