Write-Host "=================================================" -ForegroundColor Green
Write-Host ""
Write-Host "     Building Logi VHID Manager Project" -ForegroundColor Green
Write-Host ""
Write-Host "=================================================" -ForegroundColor Green

# Helper function to run cargo commands and check for errors
function Invoke-Cargo {
    param(
        [string]$Step,
        [string[]]$Arguments
    )
    
    Write-Host ""
    Write-Host "[$Step] Compiling with 'cargo $($Arguments -join ' ')'..." -ForegroundColor Yellow
    
    # Execute cargo with the arguments spread out
    cargo $Arguments
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Command failed." -ForegroundColor Red
        # Pause to allow user to see the error before exiting
        Read-Host "Press Enter to exit..."
        exit 1
    }
    Write-Host "[SUCCESS] Step completed." -ForegroundColor Green
}

Invoke-Cargo "1/4: Main Executable (Debug)" @("build")
Invoke-Cargo "2/4: Main Executable (Release)" @("build", "--release")
Invoke-Cargo "3/4: Library (Debug)" @("build", "--lib")
Invoke-Cargo "4/4: Library (Release)" @("build", "--lib", "--release")


Write-Host ""
Write-Host "=================================================" -ForegroundColor Green
Write-Host ""
Write-Host "     Build process completed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "     - Debug builds are in: target\debug"
Write-Host "     - Release builds are in: target\release"
Write-Host ""
Write-Host "=================================================" -ForegroundColor Green

Read-Host "Press Enter to exit..."