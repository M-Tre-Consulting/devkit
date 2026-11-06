<#
.SYNOPSIS
    Packages unstripped native libraries (.so) with debug symbols into a ZIP file
    formatted for Google Play Console crash deobfuscation.

.DESCRIPTION
    Scans target/<triple>/<profile>/libdevkit.so for all compiled Android architectures,
    organizes them according to the Google Play Store native debug symbols ABI directory
    structure (arm64-v8a, x86_64, armeabi-v7a, x86), and packages them into
    native-debug-symbols.zip.

.PARAMETER OutputZip
    Path to the output symbols ZIP file (default: target/release/bundle/native-debug-symbols.zip).
.PARAMETER Profile
    Cargo build profile to search: "release" (default) or "debug".
.PARAMETER LibName
    Name of the native library file (default: libdevkit.so).

.EXAMPLE
    .\scripts\package-symbols.ps1

.EXAMPLE
    .\scripts\package-symbols.ps1 -Profile debug
#>

[CmdletBinding()]
param(
    [string]$OutputZip = "",
    [string]$Profile = "release",
    [string]$LibName = "libdevkit.so"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

# Determine project root
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $ProjectRoot

if (-not $OutputZip) {
    $OutputZip = Join-Path $ProjectRoot "target\$Profile\bundle\native-debug-symbols.zip"
} else {
    $OutputZip = [System.IO.Path]::GetFullPath($OutputZip)
}

Write-Host "=====================================================" -ForegroundColor Cyan
Write-Host " Packaging Native Debug Symbols for Google Play" -ForegroundColor Cyan
Write-Host "=====================================================" -ForegroundColor Cyan
Write-Host "Profile:     $Profile"
Write-Host "Output ZIP:  $OutputZip"

# Architecture mapping: Rust target triple -> Android ABI folder
$ArchMap = [ordered]@{
    "aarch64-linux-android"   = "arm64-v8a"
    "x86_64-linux-android"    = "x86_64"
    "armv7-linux-androideabi" = "armeabi-v7a"
    "i686-linux-android"      = "x86"
}

$foundLibs = @{}

foreach ($triple in $ArchMap.Keys) {
    $abi = $ArchMap[$triple]
    # Check standard cargo target paths
    $candidatePaths = @(
        (Join-Path $ProjectRoot "target\$triple\$Profile\$LibName"),
        (Join-Path $ProjectRoot "target\$triple\$Profile\deps\$LibName")
    )

    foreach ($cand in $candidatePaths) {
        if (Test-Path $cand -PathType Leaf) {
            $foundLibs[$abi] = (Resolve-Path $cand).Path
            break
        }
    }
}

# If none found for the requested profile, try fallback search
if ($foundLibs.Count -eq 0) {
    if ($Profile -eq "release") {
        Write-Warning "No unstripped libraries found under target/<triple>/release."
        Write-Warning "Checking if debug libraries exist..."
        foreach ($triple in $ArchMap.Keys) {
            $abi = $ArchMap[$triple]
            $cand = Join-Path $ProjectRoot "target\$triple\debug\$LibName"
            if (Test-Path $cand -PathType Leaf) {
                $foundLibs[$abi] = (Resolve-Path $cand).Path
            }
        }
        if ($foundLibs.Count -gt 0) {
            Write-Warning "Found debug libraries. Packaging debug symbols as fallback."
        }
    }
}

if ($foundLibs.Count -eq 0) {
    throw "Error: No unstripped '$LibName' files found in target directory. Please run 'cargo apk build --release' or 'cargo build --target <arch> --release' first."
}

# Create staging directory
$StagingDir = Join-Path ([System.IO.Path]::GetTempPath()) ("devkit_symbols_" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $StagingDir -Force | Out-Null

try {
    Write-Host "`nCollecting unstripped libraries:" -ForegroundColor Yellow
    foreach ($abi in $foundLibs.Keys) {
        $sourcePath = $foundLibs[$abi]
        $fileItem = Get-Item $sourcePath
        $sizeMb = [Math]::Round($fileItem.Length / 1MB, 2)
        Write-Host "  -> [$abi] from $sourcePath ($sizeMb MB)"

        $abiDestDir = Join-Path $StagingDir $abi
        New-Item -ItemType Directory -Path $abiDestDir -Force | Out-Null
        Copy-Item -Path $sourcePath -Destination (Join-Path $abiDestDir $LibName) -Force
    }

    # Ensure output directory exists
    $outParent = [System.IO.Path]::GetDirectoryName($OutputZip)
    if (-not (Test-Path $outParent)) {
        New-Item -ItemType Directory -Path $outParent -Force | Out-Null
    }

    if (Test-Path $OutputZip) {
        Remove-Item $OutputZip -Force
    }

    Write-Host "`nCreating ZIP archive: $OutputZip" -ForegroundColor Cyan
    $zipArchive = [System.IO.Compression.ZipFile]::Open($OutputZip, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        $filesToZip = Get-ChildItem -Path $StagingDir -Recurse -File
        $rootUri = New-Object System.Uri(($StagingDir.TrimEnd('\') + '\'))
        foreach ($f in $filesToZip) {
            $fUri = New-Object System.Uri($f.FullName)
            $relZipPath = [System.Uri]::UnescapeDataString($rootUri.MakeRelativeUri($fUri).ToString()).Replace('\', '/')
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zipArchive, $f.FullName, $relZipPath, [System.IO.Compression.CompressionLevel]::Fastest) | Out-Null
        }
    } finally {
        $zipArchive.Dispose()
    }

    $finalZip = Get-Item $OutputZip
    $zipSizeMb = [Math]::Round($finalZip.Length / 1MB, 2)

    Write-Host "`n=====================================================" -ForegroundColor Green
    Write-Host " Successfully created native debug symbols ZIP:" -ForegroundColor Green
    Write-Host " $OutputZip ($zipSizeMb MB)" -ForegroundColor Green
    Write-Host "=====================================================" -ForegroundColor Green
    Write-Host "`nTo upload to Google Play Console:"
    Write-Host "1. Go to: Release -> App bundle explorer -> Select version"
    Write-Host "2. Click 'Downloads' or 'Files & assets' tab"
    Write-Host "3. Under 'Native debug symbols', click the upload arrow and select this ZIP."
} finally {
    if (Test-Path $StagingDir) {
        Remove-Item -Recurse -Force $StagingDir -ErrorAction SilentlyContinue
    }
}
