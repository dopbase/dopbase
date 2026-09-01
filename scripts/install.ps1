[CmdletBinding()]
param(
  [string]$Version = $env:DOPBASE_VERSION,
  [string]$InstallDir = $env:DOPBASE_INSTALL_DIR,
  [string]$DownloadBaseUrl = $env:DOPBASE_DOWNLOAD_BASE_URL
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repositoryUrl = "https://github.com/dopbase/dopbase"
if ([string]::IsNullOrWhiteSpace($InstallDir)) {
  $base = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { $env:USERPROFILE }
  $InstallDir = Join-Path $base "Dopbase\bin"
}

if ([string]::IsNullOrWhiteSpace($Version)) {
  $release = Invoke-RestMethod -Uri "https://api.github.com/repos/dopbase/dopbase/releases/latest"
  $Version = [string]$release.tag_name
}
$Version = $Version.TrimStart("v")
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
  throw "dopbase installer: invalid version: $Version"
}

$archiveName = "dopbase_${Version}_windows_amd64.zip"
if ([string]::IsNullOrWhiteSpace($DownloadBaseUrl)) {
  $DownloadBaseUrl = "$repositoryUrl/releases/download/v$Version"
}
$DownloadBaseUrl = $DownloadBaseUrl.TrimEnd("/")

$temporaryDir = Join-Path ([IO.Path]::GetTempPath()) ("dopbase-install-" + [guid]::NewGuid())
$archivePath = Join-Path $temporaryDir $archiveName
$checksumsPath = Join-Path $temporaryDir "checksums.txt"
$extractDir = Join-Path $temporaryDir "extract"

function Receive-File {
  param([string]$Source, [string]$Destination)
  if ($Source.StartsWith("file://", [StringComparison]::OrdinalIgnoreCase)) {
    Copy-Item -LiteralPath ([uri]$Source).LocalPath -Destination $Destination
  } else {
    Invoke-WebRequest -Uri $Source -OutFile $Destination -UseBasicParsing
  }
}

try {
  New-Item -ItemType Directory -Path $temporaryDir | Out-Null
  Write-Host "Downloading Dopbase $Version for windows/amd64..."
  Receive-File "$DownloadBaseUrl/$archiveName" $archivePath
  Receive-File "$DownloadBaseUrl/checksums.txt" $checksumsPath

  $expectedChecksum = $null
  foreach ($line in Get-Content -LiteralPath $checksumsPath) {
    if ($line -match '^([0-9a-fA-F]{64})\s+\*?(.+)$' -and $Matches[2].Trim() -eq $archiveName) {
      $expectedChecksum = $Matches[1].ToLowerInvariant()
      break
    }
  }
  if (-not $expectedChecksum) {
    throw "dopbase installer: checksum not found for $archiveName"
  }
  $actualChecksum = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actualChecksum -ne $expectedChecksum) {
    throw "dopbase installer: checksum verification failed"
  }

  Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir
  $binary = Join-Path $extractDir "dopbase.exe"
  if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw "dopbase installer: release archive does not contain dopbase.exe"
  }

  New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
  $temporaryTarget = Join-Path $InstallDir ".dopbase.install.$PID.exe"
  $target = Join-Path $InstallDir "dopbase.exe"
  Copy-Item -LiteralPath $binary -Destination $temporaryTarget
  Move-Item -Force -LiteralPath $temporaryTarget -Destination $target

  Write-Host "Installed Dopbase $Version to $target"
  $pathEntries = $env:PATH -split ';'
  if ($InstallDir -notin $pathEntries) {
    Write-Host "Add $InstallDir to PATH before running dopbase."
  }
} finally {
  if (Test-Path -LiteralPath $temporaryDir) {
    Remove-Item -Recurse -Force -LiteralPath $temporaryDir
  }
}
