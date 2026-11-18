param(
    [Parameter(Mandatory)] [string] $Target,
    [Parameter(Mandatory)] [string] $Label,
    [Parameter(Mandatory)] [string] $Version
)

$ErrorActionPreference = 'Stop'

$msi = (Resolve-Path "dist\presswork-$($Label -replace '/', '-')-$Target.msi").Path
$log = Join-Path ([IO.Path]::GetTempPath()) 'presswork-msi.log'
$folder = Join-Path $env:ProgramFiles 'Presswork'
$program = Join-Path $folder 'presswork.exe'
$shortcut = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs\Presswork.lnk'
$failures = 0

function Invoke-Installer([string] $Action) {
    $process = Start-Process msiexec.exe -Wait -PassThru -ArgumentList @(
        $Action, "`"$msi`"", '/qn', '/norestart', '/l*v', "`"$log`""
    )
    if ($process.ExitCode -ne 0) {
        Get-Content $log -Tail 40
        throw "msiexec $Action stopped with the exit code $($process.ExitCode)."
    }
}

function Fail([string] $Message) {
    Write-Output "Failed: $Message"
    $script:failures++
}

Invoke-Installer '/i'
foreach ($file in $program, (Join-Path $folder 'LICENSE.txt'), (Join-Path $folder 'NOTICE.txt'), $shortcut) {
    if (-not (Test-Path $file)) { Fail "the installer does not install $file" }
}
$entry = Get-ItemProperty 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' |
    Where-Object { $_.DisplayName -eq 'Presswork' }
if ($entry.DisplayVersion -ne $Version) {
    Fail "the installed version is '$($entry.DisplayVersion)', not '$Version'"
}

Invoke-Installer '/x'
foreach ($file in $program, $shortcut) {
    if (Test-Path $file) { Fail "the removal does not remove $file" }
}

if ($failures -ne 0) { throw "$failures Windows package checks failed." }
Write-Output "The MSI file of Presswork $Version installs and uninstalls correctly."
