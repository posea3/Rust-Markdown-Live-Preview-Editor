#Requires -Version 7.0
param([string]$EvidenceDirectory = ('target/phase-08-profiling/runner-tests-' + [guid]::NewGuid().ToString('N')))
$ErrorActionPreference = 'Stop'
$regressionRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$regressionRunner = Join-Path $regressionRoot 'scripts/profile-phase8.ps1'
$regressionEvidence = Join-Path $regressionRoot $EvidenceDirectory
New-Item -ItemType Directory -Path $regressionEvidence -Force | Out-Null
$regressionResults = [Collections.Generic.List[object]]::new()

function Test-RunnerBehavior {
    param([string]$Name, [scriptblock]$Work)
    try {
        & $Work
        $regressionResults.Add([pscustomobject]@{ test = $Name; result = 'PASS'; detail = '' })
        Write-Output ($Name + ': PASS')
    } catch {
        $regressionResults.Add([pscustomobject]@{ test = $Name; result = 'FAIL'; detail = $_.Exception.Message })
        Write-Output ($Name + ': FAIL: ' + $_.Exception.Message)
    }
}

Test-RunnerBehavior 'reject-disjoint-run-without-overwriting-evidence' {
    $regressionReuseDir = Join-Path $regressionEvidence 'reuse'
    & $regressionRunner -Fixtures 'mixed-10kb' -Samples 1 -Warmups 0 -NoLayout -OutputDirectory $regressionReuseDir | Out-Null
    $regressionProcessHash = (Get-FileHash -LiteralPath (Join-Path $regressionReuseDir 'processes.csv')).Hash
    $regressionSummaryHash = (Get-FileHash -LiteralPath (Join-Path $regressionReuseDir 'summary.csv')).Hash
    $regressionRejected = $false
    try { & $regressionRunner -Fixtures 'mixed-100kb' -Samples 1 -Warmups 0 -NoLayout -OutputDirectory $regressionReuseDir | Out-Null } catch { $regressionRejected = $true }
    if (-not $regressionRejected) { throw 'A second disjoint fixture run was accepted in the same evidence directory' }
    if ((Get-FileHash -LiteralPath (Join-Path $regressionReuseDir 'processes.csv')).Hash -ne $regressionProcessHash -or (Get-FileHash -LiteralPath (Join-Path $regressionReuseDir 'summary.csv')).Hash -ne $regressionSummaryHash) { throw 'Prior aggregate records were modified' }
    if (Test-Path -LiteralPath (Join-Path $regressionReuseDir 'mixed-100kb.csv')) { throw 'The rejected run wrote a new fixture' }
}

Test-RunnerBehavior 'interruption-cleans-up-only-the-owned-child' {
    $script:regressionOwnedChild = $null
    $script:regressionOwnedId = $null
    $script:regressionOwnedStart = $null
    try {
        try {
            # Dot sourcing exposes the runner's exact Process object. No global
            # process enumeration, name-based termination or test hook is used.
            . $regressionRunner -Fixtures 'mixed-10mb' -Samples 1 -Warmups 0 -TimeoutSeconds 60 -NoLayout -OutputDirectory (Join-Path $regressionEvidence 'interrupt') | ForEach-Object {
                if ($_ -like '*running*') {
                    $script:regressionOwnedChild = $profileProcess
                    $script:regressionOwnedId = $profileProcess.Id
                    $script:regressionOwnedStart = $profileProcess.StartTime
                    throw [OperationCanceledException]::new('Intentional regression cancellation')
                }
            }
        } catch {
            if ($_.Exception.Message -notlike '*Intentional regression cancellation*') { throw }
        }
        if ($null -eq $regressionOwnedId) { throw 'No owned child was observed at the cancellation point' }
        $regressionStillRunning = Get-Process -Id $regressionOwnedId -ErrorAction SilentlyContinue
        if ($null -ne $regressionStillRunning -and $regressionStillRunning.StartTime -eq $regressionOwnedStart) { throw 'Owned child survived runner interruption' }
    } finally {
        if ($null -ne $regressionOwnedChild) {
            # Ensure the RED run cannot leave the intentionally orphaned child.
            try { if (-not $regressionOwnedChild.HasExited) { $regressionOwnedChild.Kill($true); $regressionOwnedChild.WaitForExit() } } catch { }
            $regressionOwnedChild.Dispose()
        }
    }
}

Test-RunnerBehavior 'powershell51-parser-sees-a-newer-runtime-requirement' {
    $regressionLegacyExe = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
    $regressionProbeOut = Join-Path $regressionEvidence 'powershell51.json'
    $regressionProbeErr = Join-Path $regressionEvidence 'powershell51.stderr.log'
    # The machine's 5.1 execution policy blocks script files. Read requirements
    # through that host's parser; do not change/bypass the execution policy.
    $regressionProbeCommand = @'
$probeTokens = $null
$probeErrors = $null
$probeAst = [Management.Automation.Language.Parser]::ParseFile('scripts/profile-phase8.ps1', [ref]$probeTokens, [ref]$probeErrors)
[pscustomobject]@{ version = $PSVersionTable.PSVersion.ToString(); required_version = [string]$probeAst.ScriptRequirements.RequiredPSVersion; parse_errors = $probeErrors.Count; kill_tree_supported = [bool]([Diagnostics.Process].GetMethods() | Where-Object { $_.Name -eq 'Kill' -and $_.GetParameters().Count -eq 1 }) } | ConvertTo-Json -Compress
'@
    $regressionProbeEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($regressionProbeCommand))
    $regressionProbeArgs = @('-NoProfile', '-EncodedCommand', $regressionProbeEncoded)
    $regressionLegacy = Start-Process -FilePath $regressionLegacyExe -ArgumentList $regressionProbeArgs -WorkingDirectory $regressionRoot -WindowStyle Hidden -RedirectStandardOutput $regressionProbeOut -RedirectStandardError $regressionProbeErr -PassThru
    try { $regressionLegacy.WaitForExit() } finally { $regressionLegacy.Dispose() }
    $regressionProbe = Get-Content -LiteralPath $regressionProbeOut -Raw | ConvertFrom-Json
    if ([string]::IsNullOrEmpty($regressionProbe.required_version) -or [version]$regressionProbe.required_version -lt [version]'7.0' -or [version]$regressionProbe.version -ge [version]$regressionProbe.required_version -or $regressionProbe.parse_errors -ne 0 -or $regressionProbe.kill_tree_supported) { throw ('5.1 is not excluded by runtime requirements: ' + ($regressionProbe | ConvertTo-Json -Compress)) }
}

$regressionResults | Export-Csv -LiteralPath (Join-Path $regressionEvidence 'results.csv') -NoTypeInformation -Encoding utf8
if ($regressionResults | Where-Object { $_.result -eq 'FAIL' }) { exit 1 }
exit 0
