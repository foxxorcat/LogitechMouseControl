# --- Configuration ---
$ProjectName = "logi_vhid_manager"
$LibName = "logi_vhid"
$DistDir = ".\dist"

# --- Script Start ---
Write-Host "=================================================" -ForegroundColor Green
Write-Host ""
Write-Host "     Building and Packaging Logi VHID Manager" -ForegroundColor Green
Write-Host ""
Write-Host "=================================================" -ForegroundColor Green

# Helper function to run cargo commands
function Invoke-Cargo {
    param(
        [string]$Step,
        [string[]]$Arguments
    )
    
    Write-Host ""
    Write-Host "[$Step] Compiling with 'cargo $($Arguments -join ' ')'..." -ForegroundColor Yellow
    
    cargo $Arguments
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Command failed." -ForegroundColor Red
        Read-Host "Press Enter to exit..."
        exit 1
    }
    Write-Host "[SUCCESS] Step completed." -ForegroundColor Green
}

# --- 1. Build the Release Artifacts ---
Invoke-Cargo "1/3: Main Executable (Release)" @("build", "--release")
Invoke-Cargo "2/3: Library (Release)" @("build", "--lib", "--release")

# --- 2. Create and Prepare Packaging Directory ---
Write-Host ""
Write-Host "[3/5] Preparing packaging directory..." -ForegroundColor Yellow

if (Test-Path $DistDir) {
    Write-Host "     - Cleaning up old '$DistDir' directory."
    Remove-Item -Recurse -Force $DistDir
}
New-Item -ItemType Directory -Path $DistDir | Out-Null
New-Item -ItemType Directory -Path (Join-Path $DistDir "examples") | Out-Null

Write-Host "[SUCCESS] Packaging directory created." -ForegroundColor Green

# --- 3. Copy All Necessary Files ---
Write-Host ""
Write-Host "[4/5] Copying files to '$DistDir'..." -ForegroundColor Yellow

$ReleaseDir = ".\target\release"

$FilesToCopy = @(
    @{ Source = Join-Path $ReleaseDir "$ProjectName.exe"; Dest = $DistDir },
    @{ Source = Join-Path $ReleaseDir "$ProjectName.pdb"; Dest = $DistDir },
    @{ Source = Join-Path $ReleaseDir "$LibName.dll";     Dest = $DistDir },
    @{ Source = Join-Path $ReleaseDir "$LibName.pdb";     Dest = $DistDir },
    @{ Source = ".\README.md";                            Dest = $DistDir },
    @{ Source = ".\logi_vhid.py";                         Dest = $DistDir }
)

foreach ($file in $FilesToCopy) {
    if (Test-Path $file.Source) {
        Copy-Item -Path $file.Source -Destination $file.Dest
        Write-Host "     - Copied $(Split-Path $file.Source -Leaf)"
    } else {
        Write-Host "     - [WARNING] Source file not found: $($file.Source)" -ForegroundColor Red
    }
}

Get-ChildItem -Path ".\" -Filter "test_*.py" | ForEach-Object {
    Copy-Item -Path $_.FullName -Destination (Join-Path $DistDir "examples")
    Write-Host "     - Copied $($_.Name) to examples\"
}

Write-Host "[SUCCESS] All files packaged." -ForegroundColor Green

# --- 4. Compress the Distribution Folder using Git Version ---
Write-Host ""
Write-Host "[5/5] Compressing distribution package..." -ForegroundColor Yellow

# 尝试获取 Git 标签，如果失败则获取提交哈希
$GitVersion = $(git describe --tags --abbrev=0 2>$null)
if (-not $GitVersion) {
    Write-Host "     - No Git tag found. Falling back to commit hash."
    $GitVersion = $(git rev-parse --short HEAD 2>$null)
}

if (-not $GitVersion) {
    Write-Host "     - [WARNING] Could not determine Git version. Using 'dev' as version." -ForegroundColor Yellow
    $GitVersion = "dev"
} else {
    Write-Host "     - Using Git version: $GitVersion"
}

$ZipFileName = "$ProjectName-$GitVersion.zip"

if (Test-Path $ZipFileName) {
    Write-Host "     - Removing old archive: $ZipFileName"
    Remove-Item $ZipFileName
}

Compress-Archive -Path "$DistDir\*" -DestinationPath $ZipFileName -Force
Write-Host "[SUCCESS] Created distribution package: $ZipFileName" -ForegroundColor Green


# --- 5. Final Summary ---
Write-Host ""
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "     Packaging Complete!" -ForegroundColor Cyan
Write-Host ""
Write-Host "     Distribution package created at: $ZipFileName"
Write-Host ""
Write-Host "=================================================" -ForegroundColor Cyan

Read-Host "Press Enter to exit..."