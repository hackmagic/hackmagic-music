# HackMagic Music Player - BASS Audio Library Downloader
# Downloads BASS libraries from un4seen.com and places them in target/release/

$TARGET = "$PSScriptRoot\target\release"
$TEMP = "$env:TEMP\hackmagic-bass"

# Ensure target directory exists
New-Item -ItemType Directory -Force -Path $TARGET | Out-Null
New-Item -ItemType Directory -Force -Path $TEMP | Out-Null

function Download-Bass {
    param([string]$Url, [string]$Name)
    
    $zip = "$TEMP\$Name.zip"
    Write-Host "Downloading $Name..." -ForegroundColor Cyan
    
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $Url -OutFile $zip -UseBasicParsing -ErrorAction Stop
        
        Write-Host "Extracting $Name..." -ForegroundColor Cyan
        Expand-Archive -Path $zip -DestinationPath "$TEMP\$Name" -Force
        
        # Copy DLLs to target
        $dlls = Get-ChildItem -Path "$TEMP\$Name" -Recurse -Filter "*.dll"
        foreach ($dll in $dlls) {
            $dest = "$TARGET\$($dll.Name)"
            Copy-Item -Path $dll.FullName -Destination $dest -Force
            Write-Host "  -> $dest" -ForegroundColor Green
        }
    } catch {
        Write-Host "Failed to download $Name : $_" -ForegroundColor Red
    }
}

Write-Host "==========================================" -ForegroundColor Yellow
Write-Host " BASS Audio Library Downloader" -ForegroundColor Yellow
Write-Host "==========================================" -ForegroundColor Yellow
Write-Host ""

# Main BASS library (required)
Download-Bass -Url "https://www.un4seen.com/files/bass24.zip" -Name "bass"

# BASS_FX (recommended)
Download-Bass -Url "https://www.un4seen.com/files/bass_fx24.zip" -Name "bass_fx"

# BASS WASAPI output (optional)
Download-Bass -Url "https://www.un4seen.com/files/basswasapi24.zip" -Name "basswasapi"

# BASS MIDI (optional)
Download-Bass -Url "https://www.un4seen.com/files/bassmidi24.zip" -Name "bassmidi"

Write-Host ""
Write-Host "------------------------------------------" -ForegroundColor Yellow
Write-Host "BASS libraries downloaded to: $TARGET" -ForegroundColor Yellow

# Verify
$dlls = Get-ChildItem -Path $TARGET -Filter "bass*.dll"
if ($dlls.Count -gt 0) {
    Write-Host "Files:" -ForegroundColor Cyan
    foreach ($dll in $dlls) {
        Write-Host "  ✓ $($dll.Name)" -ForegroundColor Green
    }
} else {
    Write-Host "No BASS DLLs found in target!" -ForegroundColor Red
}

Write-Host "------------------------------------------" -ForegroundColor Yellow
Remove-Item -Path $TEMP -Recurse -Force -ErrorAction SilentlyContinue
