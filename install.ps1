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

# Download.
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$zip = Join-Path $env:TEMP $asset
Invoke-WebRequest $url -OutFile $zip

# Verify against the SHA256SUMS published with the release before extracting.
# A one-liner piped into iex has no other integrity check, so a mismatch or a
# missing entry is fatal rather than a warning.
$sumsUrl = "https://github.com/$repo/releases/download/$version/SHA256SUMS"
try {
    $sums = (Invoke-WebRequest $sumsUrl -UseBasicParsing).Content
} catch {
    Remove-Item $zip -Force
    throw "Could not download $sumsUrl to verify the release. Aborting."
}

$line = $sums -split "`n" | Where-Object { $_ -match "\s\*?$([regex]::Escape($asset))\s*$" } | Select-Object -First 1
if (-not $line) {
    Remove-Item $zip -Force
    throw "No SHA256SUMS entry for $asset. Aborting."
}

$expected = ($line -split '\s+')[0].Trim().ToLower()
$actual   = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
if ($actual -ne $expected) {
    Remove-Item $zip -Force
    throw "Checksum mismatch for $asset.`n  expected: $expected`n  actual:   $actual`nAborting."
}
Write-Host "Checksum verified ($expected)."

# Extract.
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
