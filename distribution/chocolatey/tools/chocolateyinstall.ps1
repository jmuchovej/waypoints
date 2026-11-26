$ErrorActionPreference = 'Stop'

# The NSIS installer, silent. No uninstall script: Chocolatey's Auto
# Uninstaller removes what the installer registered.
$packageArgs = @{
  packageName    = '{{ package.id }}'
  fileType       = 'exe'
  url64bit       = '{{ artifact.windows_amd64.exe.url }}'
  checksum64     = '{{ artifact.windows_amd64.exe.sha256 }}'
  checksumType64 = 'sha256'
  silentArgs     = '/S'
  validExitCodes = @(0)
}

Install-ChocolateyPackage @packageArgs
