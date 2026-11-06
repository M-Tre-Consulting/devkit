<#
.SYNOPSIS
    Builds an Android App Bundle (.aab) for DevKit on Windows without requiring WSL2.

.DESCRIPTION
    Replicates the functionality of build-aab.sh natively on Windows PowerShell.
    Compiles resources with aapt2, links protobuf resources, packages the base module
    with native libraries from the compiled APK, builds the AAB with bundletool,
    validates the bundle, and optionally signs it with jarsigner.

.PARAMETER ApkPath
    Path to the compiled input APK (default: target/release/apk/devkit.apk).
.PARAMETER OutputAab
    Path to the output .aab bundle file (default: target/release/bundle/devkit.aab).
.PARAMETER ManifestPath
    Path to AndroidManifest.xml (default: target/release/apk/AndroidManifest.xml).
.PARAMETER ResDir
    Path to Android res folder (default: res).
.PARAMETER BundletoolJar
    Path to bundletool.jar (default: ~/.bundletool/bundletool.jar; downloads if missing).
.PARAMETER KeystorePath
    Path to keystore file for signing the AAB (optional).
.PARAMETER KeystorePassword
    Keystore password (optional).
.PARAMETER KeyAlias
    Key alias (optional).
.PARAMETER KeyPassword
    Key password (optional).
.PARAMETER BuildApk
    If specified, runs 'cargo apk build --release' first.
.PARAMETER DebugProfile
    If specified, targets debug paths (target/debug/...) instead of release.

.EXAMPLE
    .\scripts\build-aab.ps1

.EXAMPLE
    .\scripts\build-aab.ps1 -DebugProfile

.EXAMPLE
    .\scripts\build-aab.ps1 -BuildApk
#>

[CmdletBinding()]
param(
    [string]$ApkPath = "",
    [string]$OutputAab = "",
    [string]$ManifestPath = "",
    [string]$ResDir = "",
    [string]$BundletoolJar = $env:BUNDLETOOL_JAR,
    [string]$KeystorePath = $(if ($env:KEYSTORE_PATH) { $env:KEYSTORE_PATH } else { $env:CARGO_APK_RELEASE_KEYSTORE }),
    [string]$KeystorePassword = $(if ($env:KEYSTORE_PASSWORD) { $env:KEYSTORE_PASSWORD } else { $env:CARGO_APK_RELEASE_KEYSTORE_PASSWORD }),
    [string]$KeyAlias = $(if ($env:KEY_ALIAS) { $env:KEY_ALIAS } else { $env:ANDROID_KEY_ALIAS }),
    [string]$KeyPassword = $(if ($env:KEY_PASSWORD) { $env:KEY_PASSWORD } else { $env:ANDROID_KEY_PASSWORD }),
    [switch]$BuildApk,
    [switch]$DebugProfile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Ensure .NET compression assemblies are loaded
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

# Determine project root from script location
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $ProjectRoot

# Determine profile (release vs debug)
$ProfileName = if ($DebugProfile) { "debug" } else { "release" }

# Set default paths if not explicitly provided
if (-not $ApkPath) {
    $ApkPath = Join-Path $ProjectRoot "target\$ProfileName\apk\devkit.apk"
    # Fallback to debug if release doesn't exist and debug exists
    if (-not (Test-Path $ApkPath) -and (Test-Path (Join-Path $ProjectRoot "target\debug\apk\devkit.apk")) -and -not $DebugProfile) {
        Write-Warning "Release APK not found, but debug APK found at target\debug\apk\devkit.apk. Using debug APK."
        $ProfileName = "debug"
        $ApkPath = Join-Path $ProjectRoot "target\debug\apk\devkit.apk"
    }
} else {
    $ApkPath = [System.IO.Path]::GetFullPath($ApkPath)
}

if (-not $OutputAab) {
    $OutputAab = Join-Path $ProjectRoot "target\$ProfileName\bundle\devkit.aab"
} else {
    $OutputAab = [System.IO.Path]::GetFullPath($OutputAab)
}

if (-not $ManifestPath) {
    $defaultManifest = Join-Path (Split-Path $ApkPath) "AndroidManifest.xml"
    if (Test-Path $defaultManifest) {
        $ManifestPath = $defaultManifest
    } else {
        $fallbackManifest = Join-Path $ProjectRoot "android\app\src\main\AndroidManifest.xml"
        if (Test-Path $fallbackManifest) {
            $ManifestPath = $fallbackManifest
        } else {
            $ManifestPath = Join-Path $ProjectRoot "target\$ProfileName\apk\AndroidManifest.xml"
        }
    }
} else {
    $ManifestPath = [System.IO.Path]::GetFullPath($ManifestPath)
}

if (-not $ResDir) {
    $ResDir = Join-Path $ProjectRoot "res"
} else {
    $ResDir = [System.IO.Path]::GetFullPath($ResDir)
}

Write-Host "=====================================================" -ForegroundColor Cyan
Write-Host " Building Android App Bundle (.aab) on Windows" -ForegroundColor Cyan
Write-Host "=====================================================" -ForegroundColor Cyan
Write-Host "Input APK:      $ApkPath"
Write-Host "Output AAB:     $OutputAab"
Write-Host "Manifest:       $ManifestPath"
Write-Host "Resources:      $ResDir"

# Optional: Build APK first if requested
if ($BuildApk) {
    Write-Host "`n--> Building APK with cargo-apk..." -ForegroundColor Yellow
    $cargoCmd = Get-Command cargo-apk -ErrorAction SilentlyContinue
    if (-not $cargoCmd) {
        throw "cargo-apk command not found. Please install it with 'cargo install cargo-apk'."
    }
    $buildArgs = @("apk", "build", "--lib")
    if (-not $DebugProfile) {
        $buildArgs += "--release"
    }
    Write-Host "Running: cargo $($buildArgs -join ' ')"
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo-apk build failed with exit code $LASTEXITCODE"
    }
}

# 1. Validate input APK and manifest
if (-not (Test-Path $ApkPath -PathType Leaf)) {
    throw "Error: APK not found at '$ApkPath'. Build the APK first (e.g. run with -BuildApk or 'cargo apk build --release')."
}

if (-not (Test-Path $ManifestPath -PathType Leaf)) {
    throw "Error: Manifest not found at '$ManifestPath'."
}

if (-not (Test-Path $ResDir -PathType Container)) {
    throw "Error: Resources directory not found at '$ResDir'."
}

# 2. Locate Android SDK, aapt2, and android.jar
$AndroidRoot = $env:ANDROID_HOME
if (-not $AndroidRoot -or -not (Test-Path $AndroidRoot)) {
    $AndroidRoot = $env:ANDROID_SDK_ROOT
}
if (-not $AndroidRoot -or -not (Test-Path $AndroidRoot)) {
    $localSdk = Join-Path $env:LOCALAPPDATA "Android\Sdk"
    if (Test-Path $localSdk) {
        $AndroidRoot = $localSdk
    }
}

if (-not $AndroidRoot -or -not (Test-Path $AndroidRoot)) {
    throw "Error: Android SDK not found. Please set ANDROID_HOME or ANDROID_SDK_ROOT environment variable."
}
Write-Host "Android SDK:    $AndroidRoot"

# Find aapt2.exe
$Aapt2Bin = $null
$aapt2InPath = Get-Command aapt2.exe -ErrorAction SilentlyContinue
if ($aapt2InPath) {
    $Aapt2Bin = $aapt2InPath.Source
} else {
    $buildTools = Join-Path $AndroidRoot "build-tools"
    if (Test-Path $buildTools) {
        $latestBt = Get-ChildItem -Path $buildTools -Directory | Sort-Object Name -Descending | Select-Object -First 1
        if ($latestBt) {
            $candidate = Join-Path $latestBt.FullName "aapt2.exe"
            if (Test-Path $candidate) {
                $Aapt2Bin = $candidate
            }
        }
    }
}

if (-not $Aapt2Bin -or -not (Test-Path $Aapt2Bin)) {
    throw "Error: aapt2.exe not found in PATH or Android SDK build-tools."
}
Write-Host "Using aapt2:    $Aapt2Bin"

# Find android.jar
$AndroidJar = $null
$platforms = Join-Path $AndroidRoot "platforms"
if (Test-Path $platforms) {
    $latestPlatform = Get-ChildItem -Path $platforms -Directory | Sort-Object Name -Descending | Select-Object -First 1
    if ($latestPlatform) {
        $candidateJar = Join-Path $latestPlatform.FullName "android.jar"
        if (Test-Path $candidateJar) {
            $AndroidJar = $candidateJar
        }
    }
}

if (-not $AndroidJar -or -not (Test-Path $AndroidJar)) {
    throw "Error: android.jar not found under '$platforms'."
}
Write-Host "Using SDK JAR:  $AndroidJar"

# Find Java
$JavaBin = Get-Command java.exe -ErrorAction SilentlyContinue
if (-not $JavaBin) {
    if ($env:JAVA_HOME -and (Test-Path (Join-Path $env:JAVA_HOME "bin\java.exe"))) {
        $env:PATH = "$env:JAVA_HOME\bin;$env:PATH"
        $JavaBin = Get-Command java.exe -ErrorAction SilentlyContinue
    }
}
if (-not $JavaBin) {
    throw "Error: java.exe was not found in PATH or JAVA_HOME. JDK 17+ is required."
}

# 3. Locate or download bundletool
if (-not $BundletoolJar) {
    $BundletoolJar = Join-Path $HOME ".bundletool\bundletool.jar"
}
if (-not (Test-Path $BundletoolJar)) {
    Write-Host "`nBundletool not found at '$BundletoolJar'. Downloading latest bundletool..." -ForegroundColor Yellow
    $bundleDir = Split-Path $BundletoolJar
    if (-not (Test-Path $bundleDir)) {
        New-Item -ItemType Directory -Path $bundleDir -Force | Out-Null
    }
    $bundletoolVersion = "1.18.3"
    $downloadUrl = "https://github.com/google/bundletool/releases/download/$bundletoolVersion/bundletool-all-$bundletoolVersion.jar"
    Write-Host "Downloading from $downloadUrl..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13
    Invoke-WebRequest -Uri $downloadUrl -OutFile $BundletoolJar
}
Write-Host "Using bundletool: $BundletoolJar"

# 4. Prepare temporary staging directory
$BuildTmp = Join-Path ([System.IO.Path]::GetTempPath()) ("devkit_bundle_" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $BuildTmp -Force | Out-Null

try {
    Write-Host "`n--> Step 1: Compiling resources..." -ForegroundColor Cyan
    $compiledRes = Join-Path $BuildTmp "compiled_res.zip"
    & $Aapt2Bin compile --dir "$ResDir" -o "$compiledRes"
    if ($LASTEXITCODE -ne 0) {
        throw "aapt2 compile failed with exit code $LASTEXITCODE"
    }

    Write-Host "--> Step 2: Linking protobuf resources..." -ForegroundColor Cyan
    $protoResApk = Join-Path $BuildTmp "proto_res.apk"
    & $Aapt2Bin link `
        --proto-format `
        -o "$protoResApk" `
        -I "$AndroidJar" `
        --manifest "$ManifestPath" `
        --auto-add-overlay `
        "$compiledRes"
    if ($LASTEXITCODE -ne 0) {
        throw "aapt2 link failed with exit code $LASTEXITCODE"
    }

    Write-Host "--> Step 3: Assembling base bundle module..." -ForegroundColor Cyan
    $baseDir = Join-Path $BuildTmp "base"
    $baseManifestDir = Join-Path $baseDir "manifest"
    New-Item -ItemType Directory -Path $baseManifestDir -Force | Out-Null

    # Extract proto_res.apk
    $protoExtracted = Join-Path $BuildTmp "proto_extracted"
    [System.IO.Compression.ZipFile]::ExtractToDirectory($protoResApk, $protoExtracted)

    Move-Item -Path (Join-Path $protoExtracted "AndroidManifest.xml") -Destination (Join-Path $baseManifestDir "AndroidManifest.xml") -Force
    Move-Item -Path (Join-Path $protoExtracted "resources.pb") -Destination (Join-Path $baseDir "resources.pb") -Force
    if (Test-Path (Join-Path $protoExtracted "res")) {
        Move-Item -Path (Join-Path $protoExtracted "res") -Destination (Join-Path $baseDir "res") -Force
    }
    Remove-Item -Recurse -Force $protoExtracted

    # Extract native libraries from the compiled APK
    Write-Host "--> Extracting native libraries from APK..."
    $apkZip = [System.IO.Compression.ZipFile]::OpenRead($ApkPath)
    try {
        $extractedLibs = 0
        foreach ($entry in $apkZip.Entries) {
            if ($entry.FullName.StartsWith("lib/") -and -not [string]::IsNullOrEmpty($entry.Name)) {
                $destFile = Join-Path $baseDir ($entry.FullName.Replace('/', [System.IO.Path]::DirectorySeparatorChar))
                $destSubDir = [System.IO.Path]::GetDirectoryName($destFile)
                if (-not (Test-Path $destSubDir)) {
                    New-Item -ItemType Directory -Path $destSubDir -Force | Out-Null
                }
                [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $destFile, $true)
                $extractedLibs++
            }
        }
        Write-Host "    Extracted $extractedLibs native library files."
    } finally {
        $apkZip.Dispose()
    }

    Write-Host "--> Step 4: Packaging module zip..." -ForegroundColor Cyan
    $baseZip = Join-Path $BuildTmp "base.zip"
    $zipArchive = [System.IO.Compression.ZipFile]::Open($baseZip, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        $allFiles = Get-ChildItem -Path $baseDir -Recurse -File
        $rootUri = New-Object System.Uri(($baseDir.TrimEnd('\') + '\'))
        foreach ($file in $allFiles) {
            $fileUri = New-Object System.Uri($file.FullName)
            # CRITICAL FOR BUNDLETOOL: zip entry paths MUST use forward slashes ('/')
            $relativeZipPath = [System.Uri]::UnescapeDataString($rootUri.MakeRelativeUri($fileUri).ToString()).Replace('\', '/')
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zipArchive, $file.FullName, $relativeZipPath) | Out-Null
        }
    } finally {
        $zipArchive.Dispose()
    }

    Write-Host "--> Step 5: Building App Bundle with bundletool..." -ForegroundColor Cyan
    $outputDir = [System.IO.Path]::GetDirectoryName($OutputAab)
    if (-not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }

    & java -jar "$BundletoolJar" build-bundle `
        --modules="$baseZip" `
        --output="$OutputAab" `
        --overwrite
    if ($LASTEXITCODE -ne 0) {
        throw "bundletool build-bundle failed with exit code $LASTEXITCODE"
    }

    Write-Host "--> Step 6: Validating App Bundle..." -ForegroundColor Cyan
    & java -jar "$BundletoolJar" validate --bundle="$OutputAab"
    if ($LASTEXITCODE -ne 0) {
        throw "bundletool validate failed with exit code $LASTEXITCODE"
    }

    # Optional: Sign with jarsigner if keystore is provided
    if ($KeystorePath -and (Test-Path $KeystorePath -PathType Leaf)) {
        $effectiveStorePass = if ($KeystorePassword) { $KeystorePassword } else { "" }
        $effectiveAlias = if ($KeyAlias) { $KeyAlias } else { "" }
        $effectiveKeyPass = if ($KeyPassword) { $KeyPassword } elseif ($effectiveStorePass) { $effectiveStorePass } else { "" }

        if ($effectiveAlias) {
            Write-Host "--> Step 7: Signing AAB with jarsigner..." -ForegroundColor Cyan
            $jarSignerCmd = Get-Command jarsigner.exe -ErrorAction SilentlyContinue
            if (-not $jarSignerCmd) {
                if ($env:JAVA_HOME -and (Test-Path (Join-Path $env:JAVA_HOME "bin\jarsigner.exe"))) {
                    $jarSignerCmd = Join-Path $env:JAVA_HOME "bin\jarsigner.exe"
                }
            }

            if ($jarSignerCmd) {
                & $jarSignerCmd `
                    -keystore "$KeystorePath" `
                    -storepass "$effectiveStorePass" `
                    -keypass "$effectiveKeyPass" `
                    "$OutputAab" `
                    "$effectiveAlias"
                if ($LASTEXITCODE -ne 0) {
                    throw "jarsigner failed with exit code $LASTEXITCODE"
                }
                & $jarSignerCmd -verify "$OutputAab"
                Write-Host "AAB signing and verification complete." -ForegroundColor Green
            } else {
                Write-Warning "jarsigner.exe not found. Keystore was provided, but AAB could not be signed."
            }
        }
    }

    $finalItem = Get-Item $OutputAab
    $sizeMb = [Math]::Round($finalItem.Length / 1MB, 2)
    Write-Host "`n=====================================================" -ForegroundColor Green
    Write-Host " Successfully built AAB at: $OutputAab ($sizeMb MB)" -ForegroundColor Green
    Write-Host "=====================================================" -ForegroundColor Green

} finally {
    if (Test-Path $BuildTmp) {
        Remove-Item -Recurse -Force $BuildTmp -ErrorAction SilentlyContinue
    }
}
