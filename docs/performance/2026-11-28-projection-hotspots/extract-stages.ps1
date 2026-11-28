#Requires -Version 7.0
param([Parameter(Mandatory)][string]$Directory)
$ErrorActionPreference = 'Stop'
$rows = @()
$censored = @()
foreach ($process in (Import-Csv -LiteralPath (Join-Path $Directory 'processes.csv'))) {
    $outer = $null
    $pending = $null
    $transcript = [IO.File]::ReadAllText((Join-Path ([IO.Path]::GetFullPath($Directory)) ($process.fixture + '.stages.log')))
    $lines = @($transcript -split '\r?\n')
    $lastLine = $lines.Count - 1
    while ($lastLine -ge 0 -and $lines[$lastLine] -eq '') { $lastLine-- }
    for ($lineIndex = 0; $lineIndex -le $lastLine; $lineIndex++) {
        $line = $lines[$lineIndex]
        if ($line -eq '') { continue }
        $parts = $line -split ','
        if ($parts[0] -eq 'begin') {
            if ($null -ne $pending) { throw 'New outer stage with incomplete inner stage' }
            $outer = $parts
        } elseif ($parts[0] -eq 'projection-begin') {
            if ($parts.Count -ne 3 -or $parts[1] -notmatch '^[0-9]+$' -or [string]::IsNullOrEmpty($parts[2])) { throw 'Malformed inner begin' }
            if ($null -eq $outer -or $outer[5] -ne 'projection' -or $null -ne $pending) { throw 'Unassociated/nested inner stage' }
            $pending = $parts
        } elseif ($parts[0] -eq 'projection-end') {
            if ($null -eq $pending) { throw 'Mismatched inner completion' }
            $unterminated = $lineIndex -eq $lastLine -and -not $transcript.EndsWith("`n", [StringComparison]::Ordinal)
            $interruptedTail = $lineIndex -eq $lastLine -and $process.status -in @('timeout', 'memory_limit', 'failed')
            $matchingPrefix = $parts.Count -le 4
            if ($parts.Count -ge 2) {
                $matchingPrefix = $matchingPrefix -and $(if ($unterminated) { $pending[1].StartsWith($parts[1], [StringComparison]::Ordinal) } else { $pending[1] -ceq $parts[1] })
            }
            if ($parts.Count -ge 3) {
                $matchingPrefix = $matchingPrefix -and $(if ($unterminated) { $pending[2].StartsWith($parts[2], [StringComparison]::Ordinal) } else { $pending[2] -ceq $parts[2] })
            }
            # A killed writer can leave an empty duration or a numeric prefix
            # without its newline. Neither confirms a completed observation.
            $missingDuration = $parts.Count -lt 4 -or $parts[3] -eq ''
            $possibleDurationPrefix = $parts.Count -lt 4 -or $parts[3] -match '^[0-9]*$'
            if ($interruptedTail -and $matchingPrefix -and ($missingDuration -or ($unterminated -and $possibleDurationPrefix))) { continue }
            if ($parts.Count -ne 4) { throw 'Malformed inner completion' }
            if ($null -eq $pending -or $pending[1] -ne $parts[1] -or $pending[2] -ne $parts[2]) { throw 'Mismatched inner completion' }
            $duration = 0L
            if ($unterminated -or $parts[3] -notmatch '^[0-9]+$' -or -not [long]::TryParse($parts[3], [Globalization.NumberStyles]::None, [Globalization.CultureInfo]::InvariantCulture, [ref]$duration)) { throw 'Invalid inner duration/completion' }
            $rows += [pscustomobject]@{ fixture = $outer[1]; kind = $outer[2]; sample = $outer[3]; phase = $outer[4]; call = $parts[1]; stage = $parts[2]; ns = $duration }
            $pending = $null
        }
    }
    if ($null -ne $pending) {
        if ($process.status -eq 'completed') { throw 'Completed process has incomplete inner stage' }
        $censored += [pscustomobject]@{ fixture = $outer[1]; kind = $outer[2]; sample = $outer[3]; phase = $outer[4]; call = $pending[1]; stage = $pending[2] }
    }
}
if (-not $rows.Count -and -not $censored.Count) { throw 'No internal projection stages' }
if (@($rows | Group-Object fixture,kind,sample,phase,stage | Where-Object Count -gt 1).Count) { throw 'Duplicate inner identity' }
if ($rows.Count) {
    $rows | Export-Csv -LiteralPath (Join-Path $Directory 'projection-stages.csv') -NoTypeInformation -Encoding utf8
} else {
    '"fixture","kind","sample","phase","call","stage","ns"' | Set-Content -LiteralPath (Join-Path $Directory 'projection-stages.csv') -Encoding utf8
}
$summary = @(foreach ($group in ($rows | Where-Object kind -eq 'sample' | Group-Object fixture,phase,stage)) {
    $values = @($group.Group | ForEach-Object { [double]$_.ns / 1e6 } | Sort-Object)
    $n = $values.Count
    $middle = [int][Math]::Floor($n / 2)
    $median = if ($n % 2) { $values[$middle] } else { ($values[$middle - 1] + $values[$middle]) / 2 }
    $first = $group.Group[0]
    [pscustomobject]@{ fixture = $first.fixture; phase = $first.phase; stage = $first.stage; completed_samples = $n; min_ms = $values[0]; median_ms = $median; max_ms = $values[-1] }
})
if ($summary.Count) {
    $summary | Sort-Object fixture,phase,stage | Export-Csv -LiteralPath (Join-Path $Directory 'projection-summary.csv') -NoTypeInformation -Encoding utf8
} else {
    '"fixture","phase","stage","completed_samples","min_ms","median_ms","max_ms"' | Set-Content -LiteralPath (Join-Path $Directory 'projection-summary.csv') -Encoding utf8
}
if ($censored.Count) { $censored | Export-Csv -LiteralPath (Join-Path $Directory 'projection-censored.csv') -NoTypeInformation -Encoding utf8 }
[pscustomobject]@{ directory = $Directory; completed_inner_calls = $rows.Count; summary_groups = @($summary).Count; censored_inner_calls = $censored.Count } | ConvertTo-Json -Compress
