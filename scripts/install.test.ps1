$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$rootDir = Split-Path -Parent $PSScriptRoot
$testDir = Join-Path ([IO.Path]::GetTempPath()) ("dopbase-installer-test-" + [guid]::NewGuid())
$releaseDir = Join-Path $testDir "release"
$payloadDir = Join-Path $testDir "payload"
$installDir = Join-Path $testDir "install"
$version = "0.0.12"
$archiveName = "dopbase_${version}_windows_amd64.zip"
$archivePath = Join-Path $releaseDir $archiveName

try {
  New-Item -ItemType Directory -Path $releaseDir, $payloadDir, $installDir | Out-Null
  $payload = "dopbase $version"
  Set-Content -NoNewline -LiteralPath (Join-Path $payloadDir "dopbase.exe") -Value $payload
  Compress-Archive -LiteralPath (Join-Path $payloadDir "dopbase.exe") -DestinationPath $archivePath
  $checksum = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
  Set-Content -LiteralPath (Join-Path $releaseDir "checksums.txt") -Value "$checksum  $archiveName"

  & (Join-Path $rootDir "scripts\install.ps1") `
    -Version "v$version" `
    -InstallDir $installDir `
    -DownloadBaseUrl ([uri]$releaseDir).AbsoluteUri

  $installed = Join-Path $installDir "dopbase.exe"
  if (-not (Test-Path -LiteralPath $installed -PathType Leaf)) {
    throw "installer test: dopbase.exe was not installed"
  }
  if ((Get-Content -Raw -LiteralPath $installed) -ne $payload) {
    throw "installer test: installed payload does not match"
  }

  Set-Content -LiteralPath (Join-Path $releaseDir "checksums.txt") -Value ("0" * 64 + "  $archiveName")
  $acceptedInvalidChecksum = $true
  try {
    & (Join-Path $rootDir "scripts\install.ps1") `
      -Version $version `
      -InstallDir $installDir `
      -DownloadBaseUrl ([uri]$releaseDir).AbsoluteUri
  } catch {
    $acceptedInvalidChecksum = $false
  }
  if ($acceptedInvalidChecksum) {
    throw "installer test: invalid checksum was accepted"
  }

  $acceptedInvalidVersion = $true
  try {
    & (Join-Path $rootDir "scripts\install.ps1") -Version "not-a-version" -InstallDir $installDir
  } catch {
    $acceptedInvalidVersion = $false
  }
  if ($acceptedInvalidVersion) {
    throw "installer test: invalid version was accepted"
  }

  Write-Host "PowerShell installer tests passed"
} finally {
  if (Test-Path -LiteralPath $testDir) {
    Remove-Item -Recurse -Force -LiteralPath $testDir
  }
}
