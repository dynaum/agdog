# agdog installer for Windows.
#
#   irm https://raw.githubusercontent.com/dynaum/agdog/master/install.ps1 | iex
#
# Downloads the latest release, installs agdog.exe to %LOCALAPPDATA%\agdog,
# and adds that folder to your user PATH.

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repo   = 'dynaum/agdog'
$target = 'x86_64-pc-windows-msvc'
$dest   = Join-Path $env:LOCALAPPDATA 'agdog'

Write-Host 'Installing agdog...'

# Resolve the latest release tag from the GitHub API.
$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$version = $release.tag_name
$asset   = "agdog-$version-$target.zip"
$url     = "https://github.com/$repo/releases/download/$version/$asset"

# Download and extract.
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$zip = Join-Path $env:TEMP $asset
Invoke-WebRequest $url -OutFile $zip
Expand-Archive -Path $zip -DestinationPath $dest -Force
Remove-Item $zip -Force

# Add to the user PATH if it isn't already there.
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$dest*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$dest", 'User')
    $env:Path += ";$dest"
    Write-Host "Added $dest to your PATH (restart your terminal to pick it up)."
}

Write-Host "agdog $version installed to $dest. Run: agdog"
