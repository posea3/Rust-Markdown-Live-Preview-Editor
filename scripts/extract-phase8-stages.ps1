#Requires -Version 7.0
param(
    [Parameter(Mandatory)][string]$Directory,
    [ValidateSet('extension','projection','parse')][string]$Subsystem = 'extension',
    [string[]]$ExpectedStages = @()
)
$outerStage = if ($Subsystem -eq 'extension') { 'scan' } else { $Subsystem }
if (-not $PSBoundParameters.ContainsKey('ExpectedStages')) {
    $ExpectedStages = if ($Subsystem -eq 'extension') {
        @('validation','source_text','protected_ranges','scanner_0','candidate_filter_0','pending_sort','conflict_merge','final_sort')
    } elseif ($Subsystem -eq 'parse') {
        @('source_text','parser_init','decode_events','build_tree')
    } else {
        @('validation','source_text','reveal_groups','conceal_spans','structural_padding','builtin_candidates','extension_candidates','normalize_ownership','shadow_filter','overlap_check','map','materialize','project_blocks','project_styles','project_widgets')
    }
}
if (-not $ExpectedStages.Count) { throw 'ExpectedStages must describe a complete invocation' }
$ErrorActionPreference = 'Stop'
$rows = @()
$censored = @()
$completedOuter = @()
$statuses = @{}
$unidentifiedBegins = 0
$lastOpenKeys = @{}
foreach ($process in (Import-Csv -LiteralPath (Join-Path $Directory 'processes.csv'))) {
    $statuses[$process.fixture] = $process.status
    $completedOuter += @(Import-Csv -LiteralPath (Join-Path $Directory ($process.fixture + '.csv')) | Where-Object { $_.stage -eq $outerStage -and $_.kind -in @('sample','warmup') })
    $outer = $null
    $pending = $null
    $invocationCall = $null
    $nextStage = 0
    $transcript = [IO.File]::ReadAllText((Join-Path ([IO.Path]::GetFullPath($Directory)) ($process.fixture + '.stages.log')))
    $lines = @($transcript -split '\r?\n')
    $lastLine = $lines.Count - 1
    while ($lastLine -ge 0 -and $lines[$lastLine] -eq '') { $lastLine-- }
    for ($lineIndex = 0; $lineIndex -le $lastLine; $lineIndex++) {
        $line = $lines[$lineIndex]
        if ($line -eq '') { continue }
        $parts = $line -split ','
        $unterminated = $lineIndex -eq $lastLine -and -not $transcript.EndsWith("`n", [StringComparison]::Ordinal)
        $interruptedTail = $lineIndex -eq $lastLine -and $process.status -in @('timeout', 'memory_limit', 'failed')
        $beginTag = $Subsystem + '-begin'
        $tornBegin = $interruptedTail -and $unterminated -and $beginTag.StartsWith($parts[0], [StringComparison]::Ordinal) -and ($null -eq $outer -or $outer[5] -eq $outerStage)
        if ($tornBegin -and $parts[0] -cne $beginTag) {
            # A short tag can prefix a begin, an outstanding end, or parse
            # statistics after the final stage. Prefer the known context; do
            # not invent a new begin from another subsystem's interrupted log.
            $possibleEnd = ($Subsystem + '-end').StartsWith($parts[0], [StringComparison]::Ordinal)
            $possibleStats = $Subsystem -eq 'parse' -and 'parse-stats'.StartsWith($parts[0], [StringComparison]::Ordinal)
            if (($null -ne $pending -and $possibleEnd) -or ($nextStage -ge $ExpectedStages.Count -and $possibleStats)) { $tornBegin = $false }
        }
        if ($parts[0] -eq 'begin') {
            if ($null -ne $pending) { throw 'New outer stage with incomplete inner stage' }
            $outer = $parts
            $invocationCall = $null
            $nextStage = 0
        } elseif ($parts[0] -eq $beginTag -or $tornBegin) {
            if ($null -eq $outer -or $outer[5] -ne $outerStage -or $null -ne $pending) { throw 'Unassociated/nested inner stage' }
            if ($nextStage -ge $ExpectedStages.Count) { throw 'Extra inner stage after complete invocation' }
            $expectedStage = $ExpectedStages[$nextStage]
            $completeIdentity = $parts.Count -eq 3 -and $parts[1] -match '^[0-9]+$' -and $parts[2] -ceq $expectedStage
            if (-not $completeIdentity) {
                $callPrefix = if ($parts.Count -ge 2) { $parts[1] } else { '' }
                $stagePrefix = if ($parts.Count -ge 3) { $parts[2] } else { '' }
                $validCallPrefix = $callPrefix -match '^[0-9]*$' -and ($null -eq $invocationCall -or $invocationCall.StartsWith($callPrefix, [StringComparison]::Ordinal))
                if ($tornBegin -and $parts.Count -le 3 -and $validCallPrefix -and $expectedStage.StartsWith($stagePrefix, [StringComparison]::Ordinal)) {
                    # No complete stage identity was observed: preserve earlier
                    # measurements, count the unknown tail, invent no censor ID.
                    $unidentifiedBegins++
                    continue
                }
                throw 'Malformed/out-of-order inner begin'
            }
            if ($null -ne $invocationCall -and $invocationCall -cne $parts[1]) { throw 'Inner invocation changed call ID' }
            $invocationCall = $parts[1]
            $pending = $parts
        } elseif ($parts[0] -eq ($Subsystem + '-end')) {
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
            $nextStage++
            $pending = $null
        }
    }
    if ($null -ne $pending) {
        if ($process.status -eq 'completed') { throw 'Completed process has incomplete inner stage' }
        $censored += [pscustomobject]@{ fixture = $outer[1]; kind = $outer[2]; sample = $outer[3]; phase = $outer[4]; call = $pending[1]; stage = $pending[2] }
    }
    $lastOpenKeys[$process.fixture] = if ($null -ne $outer -and $outer[5] -eq $outerStage) { @($outer[1],$outer[2],$outer[3],$outer[4]) -join '|' } else { $null }
}
if (-not $rows.Count -and -not $censored.Count -and -not $unidentifiedBegins) { throw 'No internal projection stages' }
if (@($rows | Group-Object fixture,kind,sample,phase,stage | Where-Object Count -gt 1).Count) { throw 'Duplicate inner identity' }
$completedKeys = @{}
foreach ($outerCall in $completedOuter) {
    $key = @($outerCall.fixture,$outerCall.kind,$outerCall.sample,$outerCall.phase) -join '|'
    if ($completedKeys.ContainsKey($key)) { throw 'Duplicate completed outer identity' }
    $completedKeys[$key] = $true
    $inner = @($rows | Where-Object { $_.fixture -eq $outerCall.fixture -and $_.kind -eq $outerCall.kind -and $_.sample -eq $outerCall.sample -and $_.phase -eq $outerCall.phase })
    if (@($inner | Group-Object call).Count -ne 1) { throw ('Missing/ambiguous inner invocation for completed outer ' + $key) }
    if (($inner.stage -join '|') -ne ($ExpectedStages -join '|')) { throw ('Incomplete/reordered inner invocation for completed outer ' + $key) }
}
foreach ($innerCall in ($rows | Group-Object fixture,kind,sample,phase)) {
    $first = $innerCall.Group[0]
    $key = @($first.fixture,$first.kind,$first.sample,$first.phase) -join '|'
    if (@($innerCall.Group | Group-Object call).Count -ne 1) { throw 'Multiple call IDs in one invocation' }
    if (($innerCall.Group.stage -join '|') -ne (($ExpectedStages | Select-Object -First $innerCall.Count) -join '|')) { throw 'Unordered interrupted invocation prefix' }
    if ($statuses[$first.fixture] -eq 'completed' -and -not $completedKeys.ContainsKey($key)) { throw 'Inner invocation has no completed outer call' }
    if (-not $completedKeys.ContainsKey($key) -and $innerCall.Count -lt $ExpectedStages.Count -and $lastOpenKeys[$first.fixture] -ne $key) { throw 'Incomplete invocation is no longer open' }
}
foreach ($censor in $censored) {
    $prefix = @($rows | Where-Object { $_.fixture -eq $censor.fixture -and $_.kind -eq $censor.kind -and $_.sample -eq $censor.sample -and $_.phase -eq $censor.phase })
    if ($prefix.Count -ge $ExpectedStages.Count -or $censor.stage -cne $ExpectedStages[$prefix.Count]) { throw 'Censor does not match next expected stage' }
    if ($prefix.Count -and $prefix[0].call -cne $censor.call) { throw 'Censor call ID differs from prefix' }
}
if ($rows.Count) {
    $rows | Export-Csv -LiteralPath (Join-Path $Directory ($Subsystem + '-stages.csv')) -NoTypeInformation -Encoding utf8
} else {
    '"fixture","kind","sample","phase","call","stage","ns"' | Set-Content -LiteralPath (Join-Path $Directory ($Subsystem + '-stages.csv')) -Encoding utf8
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
    $summary | Sort-Object fixture,phase,stage | Export-Csv -LiteralPath (Join-Path $Directory ($Subsystem + '-summary.csv')) -NoTypeInformation -Encoding utf8
} else {
    '"fixture","phase","stage","completed_samples","min_ms","median_ms","max_ms"' | Set-Content -LiteralPath (Join-Path $Directory ($Subsystem + '-summary.csv')) -Encoding utf8
}
if ($censored.Count) { $censored | Export-Csv -LiteralPath (Join-Path $Directory ($Subsystem + '-censored.csv')) -NoTypeInformation -Encoding utf8 }
[pscustomobject]@{ directory = $Directory; completed_inner_calls = $rows.Count; summary_groups = @($summary).Count; censored_inner_calls = $censored.Count; unidentified_begin_tails = $unidentifiedBegins } | ConvertTo-Json -Compress
