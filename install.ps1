# Crusty installer for Windows
# Usage: irm https://raw.githubusercontent.com/kevnoutsawo/crusty/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "kevnoutsawo/crusty"
$Binary = "crusty-tui.exe"
$Target = "x86_64-pc-windows-msvc"

function Main {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Crusty" "bin"

    $Tag = Get-LatestTag
    Write-Host "Latest release: $Tag"

    $Url = "https://github.com/$Repo/releases/download/$Tag/crusty-$Target.zip"
    $TmpDir = New-TemporaryDirectory

    try {
        $ZipPath = Join-Path $TmpDir "crusty.zip"

        Write-Host "Downloading $Url..."
        Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing

        Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        Move-Item -Path (Join-Path $TmpDir $Binary) -Destination (Join-Path $InstallDir $Binary) -Force

        Add-ToPath $InstallDir

        Write-Host ""
        Write-Host "Crusty $Tag installed to $InstallDir"
        Write-Host "Run 'crusty-tui' to get started."
    }
    finally {
        Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
    }
}

function Get-LatestTag {
    $Response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
    return $Response.tag_name
}

function New-TemporaryDirectory {
    $TmpPath = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $TmpPath | Out-Null
    return $TmpPath
}

function Add-ToPath($Dir) {
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$Dir*") {
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$Dir", "User")
        $env:Path = "$env:Path;$Dir"
        Write-Host "Added $Dir to your PATH."
    }
}

Main
