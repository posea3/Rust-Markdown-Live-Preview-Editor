param(
    [string[]]$Fixtures = @('mixed-10kb', 'mixed-100kb', 'mixed-1mb', 'mixed-10mb', 'single-line'),
    [ValidateRange(1,50)][int]$Samples = 5,
    [ValidateRange(0,10)][int]$Warmups = 1,
    [ValidateRange(1,3600)][int]$TimeoutSeconds = 90,
    [long]$MaxWorkingSetBytes = 1500000000,
    [string]$OutputDirectory = 'target/phase-08-profiling/baseline',
    [switch]$NoLayout,
    [switch]$SourceVisible
)
$ErrorActionPreference = 'Stop'
$profileRoot = Split-Path -Parent $PSScriptRoot
$profileBinary = Join-Path $profileRoot 'target/release/examples/phase8_profile.exe'
if (-not (Test-Path -LiteralPath $profileBinary)) { throw 'Build first: cargo build --release -p mdedit-ime-lab --example phase8_profile' }
$profileOutput = if ([IO.Path]::IsPathRooted($OutputDirectory)) { $OutputDirectory } else { Join-Path $profileRoot $OutputDirectory }
New-Item -ItemType Directory -Path $profileOutput -Force | Out-Null
$profileCases = @('mixed-10kb', 'mixed-100kb', 'mixed-1mb', 'mixed-10mb', 'single-line')
$profileRecords = @()
foreach ($profileFixture in $Fixtures) {
    if ($profileFixture -notin $profileCases) { throw ('Unknown fixture: ' + $profileFixture) }
    $profileCsv = Join-Path $profileOutput ($profileFixture + '.csv')
    $profileLog = Join-Path $profileOutput ($profileFixture + '.stages.log')
    if ((Test-Path -LiteralPath $profileCsv) -or (Test-Path -LiteralPath $profileLog)) { throw ('Output already exists; select a fresh directory: ' + $profileFixture) }
    $profileArgs = @($profileFixture, '--samples', [string]$Samples, '--warmups', [string]$Warmups, '--settle-ms', '200')
    if ($NoLayout) { $profileArgs += '--no-layout' }
    if ($SourceVisible) { $profileArgs += '--source-visible' }
    $profileWatch = [Diagnostics.Stopwatch]::StartNew()
    $profileProcess = Start-Process -FilePath $profileBinary -ArgumentList $profileArgs -WorkingDirectory $profileRoot -WindowStyle Hidden -RedirectStandardOutput $profileCsv -RedirectStandardError $profileLog -PassThru
    $profilePeak = 0L
    $profileStatus = 'completed'
    $profileNextProgress = 10
    while (-not $profileProcess.HasExited) {
        $profileProcess.Refresh()
        if ($profileProcess.HasExited) { break }
        $profilePeak = [Math]::Max($profilePeak, $profileProcess.PeakWorkingSet64)
        if ($profileWatch.Elapsed.TotalSeconds -ge $TimeoutSeconds) { $profileStatus = 'timeout' }
        elseif ($profileProcess.WorkingSet64 -gt $MaxWorkingSetBytes) { $profileStatus = 'memory_limit' }
        if ($profileStatus -ne 'completed') {
            # Only the process created immediately above (and its children).
            $profileProcess.Kill($true)
            break
        }
        if ($profileWatch.Elapsed.TotalSeconds -ge $profileNextProgress) {
            Write-Output ($profileFixture + ': running ' + [int]$profileWatch.Elapsed.TotalSeconds + 's, peak ' + [Math]::Round($profilePeak / 1MB, 1) + ' MiB')
            $profileNextProgress += 10
        }
        Start-Sleep -Milliseconds 20
    }
    $profileProcess.WaitForExit()
    $profileWatch.Stop()
    $profileExit = $profileProcess.ExitCode
    if ($profileStatus -eq 'completed' -and $profileExit -ne 0) { $profileStatus = 'failed' }
    $profileLastStage = Get-Content -LiteralPath $profileLog | Where-Object { $_ -like 'begin,*' } | Select-Object -Last 1
    $profileRecords += [pscustomobject]@{ fixture = $profileFixture; status = $profileStatus; exit_code = $profileExit; elapsed_ms = [Math]::Round($profileWatch.Elapsed.TotalMilliseconds, 1); observed_peak_working_set_bytes = $profilePeak; samples_requested = $Samples; warmups = $Warmups; timeout_seconds = $TimeoutSeconds; max_working_set_bytes = $MaxWorkingSetBytes; layout = (-not $NoLayout); source_visible = [bool]$SourceVisible; last_stage = $profileLastStage }
    $profileRecords | Export-Csv -LiteralPath (Join-Path $profileOutput 'processes.csv') -NoTypeInformation -Encoding utf8
    Write-Output ($profileFixture + ': ' + $profileStatus + ', ' + [Math]::Round($profileWatch.Elapsed.TotalSeconds, 2) + 's, peak ' + [Math]::Round($profilePeak / 1MB, 1) + ' MiB')
    $profileProcess.Dispose()
}
$profileTimingRows = foreach ($profileFixture in $Fixtures) {
    Import-Csv -LiteralPath (Join-Path $profileOutput ($profileFixture + '.csv')) | Where-Object { $_.kind -eq 'sample' }
}
$profileSummary = foreach ($profileGroup in ($profileTimingRows | Group-Object fixture,phase,stage)) {
    $profileValues = @($profileGroup.Group | ForEach-Object { [double]$_.ns / 1000000.0 } | Sort-Object)
    $profileCount = $profileValues.Count
    $profileMiddle = [int][Math]::Floor($profileCount / 2)
    $profileMedian = if ($profileCount % 2) { $profileValues[$profileMiddle] } else { ($profileValues[$profileMiddle - 1] + $profileValues[$profileMiddle]) / 2 }
    $profileFirst = $profileGroup.Group[0]
    [pscustomobject]@{ fixture = $profileFirst.fixture; phase = $profileFirst.phase; stage = $profileFirst.stage; completed_samples = $profileCount; min_ms = $profileValues[0]; median_ms = $profileMedian; max_ms = $profileValues[-1] }
}
$profileSummary | Sort-Object fixture,phase,stage | Export-Csv -LiteralPath (Join-Path $profileOutput 'summary.csv') -NoTypeInformation -Encoding utf8
