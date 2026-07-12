param(
    [string]$InstallDir,
    [switch]$Ytdlp,
    [switch]$Aria2,
    [switch]$Ffmpeg
)

$ErrorActionPreference = "Stop"
$BinDir = Join-Path $InstallDir "bin"
if (!(Test-Path $BinDir)) {
    New-Item -ItemType Directory -Force -Path $BinDir
}

if ($Ytdlp) {
    Write-Host "Downloading yt-dlp..."
    $YtdlpUrl = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    $YtdlpPath = Join-Path $BinDir "yt-dlp.exe"
    Invoke-WebRequest -Uri $YtdlpUrl -OutFile $YtdlpPath -UseBasicParsing
}

if ($Aria2) {
    Write-Host "Downloading aria2c..."
    $Aria2Url = "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-win-64bit-build1.zip"
    $Aria2Zip = Join-Path $env:TEMP "aria2.zip"
    Invoke-WebRequest -Uri $Aria2Url -OutFile $Aria2Zip -UseBasicParsing
    Expand-Archive -Path $Aria2Zip -DestinationPath $env:TEMP -Force
    $Aria2Folder = Join-Path $env:TEMP "aria2-1.37.0-win-64bit-build1"
    Copy-Item (Join-Path $Aria2Folder "aria2c.exe") $BinDir -Force
    Remove-Item $Aria2Zip -ErrorAction SilentlyContinue
    Remove-Item $Aria2Folder -Recurse -Force -ErrorAction SilentlyContinue
}

if ($Ffmpeg) {
    Write-Host "Downloading ffmpeg & ffprobe..."
    # Using GyanD's stable release links
    $FfmpegUrl = "https://github.com/GyanD/codexffmpeg/releases/download/7.0.1/ffmpeg-7.0.1-full_build.zip"
    $FfmpegZip = Join-Path $env:TEMP "ffmpeg.zip"
    Invoke-WebRequest -Uri $FfmpegUrl -OutFile $FfmpegZip -UseBasicParsing
    Expand-Archive -Path $FfmpegZip -DestinationPath $env:TEMP -Force
    $FfmpegFolder = Join-Path $env:TEMP "ffmpeg-7.0.1-full_build"
    Copy-Item (Join-Path $FfmpegFolder "bin/ffmpeg.exe") $BinDir -Force
    Copy-Item (Join-Path $FfmpegFolder "bin/ffprobe.exe") $BinDir -Force
    Remove-Item $FfmpegZip -ErrorAction SilentlyContinue
    Remove-Item $FfmpegFolder -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Setup complete."
