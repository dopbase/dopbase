[CmdletBinding()]
param(
  [switch]$FromSource,
  [string]$Version = $env:DOPBASE_VERSION,
  [string]$InstallDir = $env:DOPBASE_INSTALL_DIR,
  [string]$DownloadBaseUrl = $env:DOPBASE_DOWNLOAD_BASE_URL,
  [string]$RepositoryUrl = $env:DOPBASE_REPOSITORY_URL
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$scriptDir = $PSScriptRoot
$repoRoot = Split-Path -Parent $scriptDir

if (-not $FromSource -and @('1', 'true', 'yes') -contains $env:DOPBASE_FROM_SOURCE) {
  $FromSource = $true
}

if ([string]::IsNullOrWhiteSpace($RepositoryUrl)) {
  $RepositoryUrl = $null
  if (Get-Command git -ErrorAction SilentlyContinue) {
    $remote = git -C $repoRoot remote get-url origin 2>$null
    if ($remote -match 'github\.com[:/](?<owner>[^/]+)/(?<repo>[^/.]+)') {
      $RepositoryUrl = "https://github.com/$($Matches.owner)/$($Matches.repo)"
    }
  }
  if ([string]::IsNullOrWhiteSpace($RepositoryUrl)) {
    $RepositoryUrl = "https://github.com/dopbase/dopbase"
  }
}
if ([string]::IsNullOrWhiteSpace($InstallDir)) {
  $base = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { $env:USERPROFILE }
  $InstallDir = Join-Path $base "Dopbase\bin"
}

$autoSourceOnMiss = -not $DownloadBaseUrl -and -not $env:DOPBASE_REPOSITORY_URL

function Install-BuiltBinary {
  param([string]$BuiltBinary, [string]$TargetName)
  if (-not (Test-Path -LiteralPath $BuiltBinary -PathType Leaf)) {
    throw "dopbase installer: build did not produce $BuiltBinary"
  }
  New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
  $temporaryTarget = Join-Path $InstallDir ".dopbase.install.$PID.exe"
  $target = Join-Path $InstallDir $TargetName
  Copy-Item -LiteralPath $BuiltBinary -Destination $temporaryTarget
  Move-Item -Force -LiteralPath $temporaryTarget -Destination $target
  Write-Host "Installed Dopbase to $target"
  $pathEntries = $env:PATH -split ';'
  if ($InstallDir -notin $pathEntries) {
    Write-Host "Add $InstallDir to PATH before running dopbase."
    Write-Host ""
    Write-Host "PowerShell (this session):"
    Write-Host "  `$env:Path = `"$InstallDir;`$env:Path`""
    Write-Host ""
    Write-Host "Persist for your user account:"
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$InstallDir', 'User')"
    if ($env:DOPBASE_INSTALL_ADD_PATH -eq '1') {
      $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
      if ($userPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable('Path', "$userPath;$InstallDir", 'User')
        Write-Host "dopbase installer: appended $InstallDir to the user PATH"
      }
    }
  }
}

function Install-FromSource {
  $cargoToml = Join-Path $repoRoot "app\Cargo.toml"
  if (-not (Test-Path -LiteralPath $cargoToml)) {
    throw "dopbase installer: --FromSource requires a Dopbase repository checkout"
  }
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "dopbase installer: required command not found: cargo"
  }
  Write-Host "dopbase installer: building from source in $repoRoot..."
  $distIndex = Join-Path $repoRoot "dist\index.html"
  if (-not (Test-Path -LiteralPath $distIndex)) {
    if (-not (Get-Command bun -ErrorAction SilentlyContinue)) {
      throw "dopbase installer: dist/index.html is missing and bun was not found. Install Bun, run 'bun install' and 'bun run build:ui', then retry."
    }
    Push-Location $repoRoot
    try {
      & bun run build:ui
    } finally {
      Pop-Location
    }
  }
  Push-Location $repoRoot
  try {
    & cargo build --manifest-path app/Cargo.toml --release --locked
  } finally {
    Pop-Location
  }
  $builtBinary = Join-Path $repoRoot "app\target\release\dopbase.exe"
  Install-BuiltBinary -BuiltBinary $builtBinary -TargetName "dopbase.exe"
}

if ($FromSource) {
  Install-FromSource
  return
}

if ([string]::IsNullOrWhiteSpace($Version)) {
  $release = Invoke-RestMethod -Uri "https://api.github.com/repos/dopbase/dopbase/releases/latest"
  $Version = [string]$release.tag_name
}
$releaseTag = $Version.Trim()
if ($releaseTag -notmatch '^v?\d+\.\d+\.\d+$') {
  throw "dopbase installer: invalid release tag: $releaseTag"
}
$Version = $releaseTag.TrimStart("v")

$archiveName = "dopbase_${Version}_windows_amd64.zip"
if ([string]::IsNullOrWhiteSpace($DownloadBaseUrl)) {
  $DownloadBaseUrl = "$RepositoryUrl/releases/download/$releaseTag"
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
  try {
    Receive-File "$DownloadBaseUrl/$archiveName" $archivePath
  } catch {
    if ($autoSourceOnMiss -and (Test-Path -LiteralPath (Join-Path $repoRoot "app\Cargo.toml"))) {
      Write-Host "dopbase installer: $archiveName is not published for $releaseTag; building from source..."
      Install-FromSource
      return
    }
    throw "dopbase installer: failed to download $archiveName from $DownloadBaseUrl. Windows release archives appear after the first release that includes them. From a repository clone, run: .\scripts\install.ps1 -FromSource"
  }
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

  Install-BuiltBinary -BuiltBinary $binary -TargetName "dopbase.exe"
} finally {
  if (Test-Path -LiteralPath $temporaryDir) {
    Remove-Item -Recurse -Force -LiteralPath $temporaryDir
  }
}
