$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$rootDir = Split-Path -Parent $PSScriptRoot
$testDir = Join-Path ([IO.Path]::GetTempPath()) ("dopbase-installer-test-" + [guid]::NewGuid())
$repositoryDir = Join-Path $testDir "repository"
$releaseDir = Join-Path $repositoryDir "releases\download\0.0.12"
$legacyReleaseDir = Join-Path $repositoryDir "releases\download\v0.0.12"
$payloadDir = Join-Path $testDir "payload"
$installDir = Join-Path $testDir "install"
$version = "0.0.12"
$archiveName = "dopbase_${version}_windows_amd64.zip"
$archivePath = Join-Path $releaseDir $archiveName
$releaseUrl = ([uri]$releaseDir).AbsoluteUri
$legacyReleaseUrl = ([uri]$legacyReleaseDir).AbsoluteUri

function Invoke-RestMethod {
  param([string]$Uri)
  if ($Uri -ne "https://api.github.com/repos/dopbase/dopbase/releases/latest") {
    throw "installer test: unexpected API request: $Uri"
  }
  return [pscustomobject]@{ tag_name = $version }
}

try {
  New-Item -ItemType Directory -Path $releaseDir, $legacyReleaseDir, $payloadDir, $installDir | Out-Null
  $payload = "dopbase $version"
  Set-Content -NoNewline -LiteralPath (Join-Path $payloadDir "dopbase.exe") -Value $payload
  Compress-Archive -LiteralPath (Join-Path $payloadDir "dopbase.exe") -DestinationPath $archivePath
  $checksum = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
  Set-Content -LiteralPath (Join-Path $releaseDir "checksums.txt") -Value "$checksum  $archiveName"
  Copy-Item -LiteralPath $archivePath -Destination $legacyReleaseDir
  Copy-Item -LiteralPath (Join-Path $releaseDir "checksums.txt") -Destination $legacyReleaseDir

  & (Join-Path $rootDir "scripts\install.ps1") `
    -Version $version `
    -InstallDir $installDir `
    -DownloadBaseUrl $releaseUrl

  $installed = Join-Path $installDir "dopbase.exe"
  if (-not (Test-Path -LiteralPath $installed -PathType Leaf)) {
    throw "installer test: dopbase.exe was not installed"
  }
  if ((Get-Content -Raw -LiteralPath $installed) -ne $payload) {
    throw "installer test: installed payload does not match"
  }

  & (Join-Path $rootDir "scripts\install.ps1") `
    -InstallDir $installDir `
    -DownloadBaseUrl $releaseUrl

  & (Join-Path $rootDir "scripts\install.ps1") `
    -Version "v$version" `
    -InstallDir $installDir `
    -DownloadBaseUrl $legacyReleaseUrl

  Set-Content -LiteralPath (Join-Path $releaseDir "checksums.txt") -Value ("0" * 64 + "  $archiveName")
  $acceptedInvalidChecksum = $true
  try {
    & (Join-Path $rootDir "scripts\install.ps1") `
      -Version $version `
      -InstallDir $installDir `
      -DownloadBaseUrl $releaseUrl
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
