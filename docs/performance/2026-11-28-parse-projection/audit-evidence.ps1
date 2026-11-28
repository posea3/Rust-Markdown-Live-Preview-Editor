#Requires -Version 7.0
param([string]$Directory = $PSScriptRoot)
$ErrorActionPreference = 'Stop'
$cohorts = @('profile-before-warmed','profile-before-10mb','normal-before-warmed','normal-before-10mb','normal-after-warmed','normal-after-10mb','profile-after-warmed','profile-after-10mb','normal-before-core','normal-after-core','profile-censored')
$names = @{
    parse = @('source_text','parser_init','decode_events','build_tree')
    projection = @('validation','source_text','owner_index','reveal_grouping','reveal_materialize','conceal_spans','structural_padding','builtin_candidates','extension_candidates','normalize_ownership','shadow_filter','overlap_check','map','materialize','project_blocks','project_styles','project_widgets')
}
$outerGroups = 0; $innerGroups = 0; $fixtureFiles = 0; $outerCalls = 0; $prefixRows = 0; $censors = 0
function Audit-Summary($rows, $summary) {
    $groups = @($rows | Where-Object kind -eq 'sample' | Group-Object fixture,phase,stage)
    if ($groups.Count -ne $summary.Count) { throw 'Raw/summary group counts differ' }
    foreach ($group in $groups) {
        $first = $group.Group[0]
        $actual = @($summary | Where-Object { $_.fixture -eq $first.fixture -and $_.phase -eq $first.phase -and $_.stage -eq $first.stage })
        $values = @($group.Group | ForEach-Object { [double]$_.ns / 1e6 } | Sort-Object)
        $middle = [int][Math]::Floor($values.Count / 2)
        $median = if ($values.Count % 2) { $values[$middle] } else { ($values[$middle - 1] + $values[$middle]) / 2 }
        if ($actual.Count -ne 1 -or [int]$actual[0].completed_samples -ne $values.Count -or [Math]::Abs([double]$actual[0].min_ms - $values[0]) -gt 1e-8 -or [Math]::Abs([double]$actual[0].median_ms - $median) -gt 1e-8 -or [Math]::Abs([double]$actual[0].max_ms - $values[-1]) -gt 1e-8) { throw ('Summary mismatch: ' + $group.Name) }
    }
    return $groups.Count
}
foreach ($cohort in $cohorts) {
    $dir = Join-Path $Directory $cohort
    $processes = @(Import-Csv -LiteralPath (Join-Path $dir 'processes.csv'))
    $rows = @()
    foreach ($process in $processes) {
        if ($cohort -ne 'profile-censored' -and $process.status -ne 'completed') { throw 'Unexpected incomplete cohort' }
        $fixtureRows = @(Import-Csv -LiteralPath (Join-Path $dir ($process.fixture + '.csv')))
        if (@($fixtureRows | Group-Object fixture,kind,sample,phase,stage | Where-Object Count -gt 1).Count) { throw 'Duplicate outer identity' }
        if (@($fixtureRows | Where-Object { $_.stage -eq 'raw_fallback' -and [long]$_.items -ne 0 }).Count) { throw 'Benchmark projection fell back to raw' }
        $fixtureFiles++
        $rows += $fixtureRows
        if ($cohort -like 'normal-*' -and (Select-String -LiteralPath (Join-Path $dir ($process.fixture + '.stages.log')) -Pattern '^(parse|projection|extension)-')) { throw 'Normal build emitted internal markers' }
    }
    $outerGroups += Audit-Summary $rows @(Import-Csv -LiteralPath (Join-Path $dir 'summary.csv'))
    if ($cohort -like 'profile-*') {
        foreach ($subsystem in @('parse','projection')) {
            $innerPath = Join-Path $dir ($subsystem + '-stages.csv')
            if ($cohort -eq 'profile-censored' -and -not (Test-Path -LiteralPath $innerPath)) { continue }
            $inner = @(Import-Csv -LiteralPath $innerPath)
            $innerGroups += Audit-Summary $inner @(Import-Csv -LiteralPath (Join-Path $dir ($subsystem + '-summary.csv')))
            $completedKeys = @{}
            foreach ($outerCall in ($rows | Where-Object { $_.stage -eq $subsystem -and $_.kind -in @('warmup','sample') })) {
                $key = @($outerCall.fixture,$outerCall.kind,$outerCall.sample,$outerCall.phase) -join '|'
                $matching = @($inner | Where-Object { $_.fixture -eq $outerCall.fixture -and $_.kind -eq $outerCall.kind -and $_.sample -eq $outerCall.sample -and $_.phase -eq $outerCall.phase })
                if (@($matching | Group-Object call).Count -ne 1 -or ($matching.stage -join '|') -ne ($names[$subsystem] -join '|')) { throw ('Missing complete inner invocation: ' + $key) }
                $completedKeys[$key] = $true
                $outerCalls++
            }
            foreach ($call in ($inner | Group-Object fixture,kind,sample,phase,call)) {
                $first = $call.Group[0]
                $key = @($first.fixture,$first.kind,$first.sample,$first.phase) -join '|'
                if (-not $completedKeys.ContainsKey($key)) {
                    if ($cohort -ne 'profile-censored') { throw 'Orphan inner invocation' }
                    if (($call.Group.stage -join '|') -ne (($names[$subsystem] | Select-Object -First $call.Count) -join '|')) { throw 'Invalid interrupted prefix' }
                    $prefixRows += $call.Count
                }
            }
            $censorPath = Join-Path $dir ($subsystem + '-censored.csv')
            if (Test-Path -LiteralPath $censorPath) {
                $censored = @(Import-Csv -LiteralPath $censorPath)
                if ($cohort -ne 'profile-censored' -or $processes[0].status -ne 'timeout') { throw 'Unexpected censor' }
                foreach ($censor in $censored) {
                    if (@($inner | Where-Object { $_.fixture -eq $censor.fixture -and $_.kind -eq $censor.kind -and $_.sample -eq $censor.sample -and $_.phase -eq $censor.phase -and $_.stage -eq $censor.stage }).Count) { throw 'Fabricated censored duration' }
                }
                $censors += $censored.Count
            }
        }
    }
}
$metaMatches = 0
function Compare-Metadata($left, $right, $fixture) {
    $a = @(Import-Csv -LiteralPath (Join-Path $Directory "$left/$fixture.csv") | Where-Object kind -like 'meta-*' | Select-Object fixture,kind,sample,phase,stage,items | ConvertTo-Csv -NoTypeInformation)
    $b = @(Import-Csv -LiteralPath (Join-Path $Directory "$right/$fixture.csv") | Where-Object kind -like 'meta-*' | Select-Object fixture,kind,sample,phase,stage,items | ConvertTo-Csv -NoTypeInformation)
    if (Compare-Object $a $b) { throw "Metadata changed: $left/$right/$fixture" }
    return $a.Count - 1
}
$comparison = @(foreach ($pair in @(@('normal-before-warmed','normal-after-warmed'),@('normal-before-10mb','normal-after-10mb'),@('normal-before-core','normal-after-core'),@('profile-before-warmed','profile-after-warmed'),@('profile-before-10mb','profile-after-10mb'))) {
    foreach ($process in (Import-Csv -LiteralPath (Join-Path $Directory ($pair[0] + '/processes.csv')))) { $metaMatches += Compare-Metadata $pair[0] $pair[1] $process.fixture }
    foreach ($scope in @('outer','parse','projection')) {
        if ($scope -ne 'outer' -and $pair[0] -notlike 'profile-*') { continue }
        $file = if ($scope -eq 'outer') { 'summary.csv' } else { $scope + '-summary.csv' }
        $left = @(Import-Csv -LiteralPath (Join-Path $Directory ($pair[0] + '/' + $file)))
        $right = @(Import-Csv -LiteralPath (Join-Path $Directory ($pair[1] + '/' + $file)))
        if ($left.Count -ne $right.Count) { throw 'Paired timing group counts differ' }
        foreach ($a in $left) {
            $b = @($right | Where-Object { $_.fixture -eq $a.fixture -and $_.phase -eq $a.phase -and $_.stage -eq $a.stage })
            if ($b.Count -ne 1 -or $a.completed_samples -ne $b[0].completed_samples) { throw 'Unmatched timing group' }
            [pscustomobject]@{ scope=$scope; cohort_before=$pair[0]; cohort_after=$pair[1]; fixture=$a.fixture; phase=$a.phase; stage=$a.stage; samples_before=$a.completed_samples; samples_after=$b[0].completed_samples; before_ms=$a.median_ms; after_ms=$b[0].median_ms; before_over_after=if ([double]$b[0].median_ms -gt 0) { [double]$a.median_ms / [double]$b[0].median_ms } else { $null } }
        }
    }
})
foreach ($pair in @(@('normal-before-warmed','profile-before-warmed'),@('normal-before-10mb','profile-before-10mb'),@('normal-after-warmed','profile-after-warmed'),@('normal-after-10mb','profile-after-10mb'))) {
    foreach ($process in (Import-Csv -LiteralPath (Join-Path $Directory ($pair[0] + '/processes.csv')))) { $metaMatches += Compare-Metadata $pair[0] $pair[1] $process.fixture }
}
if ($censors -ne 1) { throw 'Expected one actual censored observation' }
$comparison | Export-Csv -LiteralPath (Join-Path $Directory 'comparison.csv') -NoTypeInformation -Encoding utf8
[pscustomobject]@{ outer_summary_groups=$outerGroups; internal_summary_groups=$innerGroups; unique_schema2_fixture_files=$fixtureFiles; validated_outer_invocations_including_warmups=$outerCalls; matching_metadata_rows=$metaMatches; timing_comparisons=$comparison.Count; completed_interrupted_prefix_rows=$prefixRows; censored_observations=$censors; fabricated_censored_durations=0 } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Directory 'verification.json') -Encoding utf8
Get-Content -LiteralPath (Join-Path $Directory 'verification.json')
$comparison | Where-Object { $_.scope -eq 'outer' -and $_.cohort_before -like 'normal-*' -and $_.phase -eq 'edit' -and $_.stage -in @('parse','projection') } | Format-Table fixture,stage,before_ms,after_ms,before_over_after -AutoSize
