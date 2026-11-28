#Requires -Version 7.0
$ErrorActionPreference = 'Stop'
$regressionRepo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$regressionRoot = Join-Path $regressionRepo ('target/extension-trace-regressions-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $regressionRoot -Force | Out-Null
$outer = 'begin,mixed-1mb,sample,0,load,scan'
$prefix = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,17', 'extension-begin,1,source_text')
$cases = @(
    @{ Name = 'torn-empty-duration'; Lines = $prefix + 'extension-end,1,source_text,'; Terminated = $true; Status = 'timeout'; Rows = 1; Censored = 'source_text' },
    @{ Name = 'torn-numeric-without-newline'; Lines = $prefix + 'extension-end,1,source_text,123'; Terminated = $false; Status = 'timeout'; Rows = 1; Censored = 'source_text' },
    @{ Name = 'censored-first-stage'; Lines = @($outer, 'extension-begin,1,validation'); Terminated = $true; Status = 'timeout'; Rows = 0; Censored = 'validation' },
    @{ Name = 'valid-zero-duration'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,0'); Terminated = $true; Status = 'completed'; Rows = 1; Censored = $null },
    @{ Name = 'reject-missing-warmup'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,17'); Terminated = $true; Status = 'completed'; Reject = $true },
    @{ Name = 'reject-negative-duration'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,-1'); Terminated = $true; Status = 'completed'; Reject = $true },
    @{ Name = 'reject-empty-completed-duration'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,'); Terminated = $true; Status = 'completed'; Reject = $true },
    @{ Name = 'reject-mismatched-completion'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,99,validation,12'); Terminated = $true; Status = 'timeout'; Reject = $true }
)
$failures = @()
foreach ($case in $cases) {
    $dir = Join-Path $regressionRoot $case.Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    [pscustomobject]@{ fixture = 'mixed-1mb'; status = $case.Status } | Export-Csv -LiteralPath (Join-Path $dir 'processes.csv') -NoTypeInformation -Encoding utf8
        $outerRows = @()
    if ($case.Status -eq 'completed') { $outerRows += [pscustomobject]@{fixture='mixed-1mb';kind='sample';sample=0;phase='load';stage='scan';ns=1;items=0} }
    if ($case.Name -eq 'reject-missing-warmup') { $outerRows += [pscustomobject]@{fixture='mixed-1mb';kind='warmup';sample=0;phase='load';stage='scan';ns=1;items=0} }
    if ($outerRows.Count) { $outerRows | Export-Csv -LiteralPath (Join-Path $dir 'mixed-1mb.csv') -NoTypeInformation -Encoding utf8 } else { 'fixture,kind,sample,phase,stage,ns,items' | Set-Content -LiteralPath (Join-Path $dir 'mixed-1mb.csv') -Encoding utf8 }
    $log = $case.Lines -join "`n"
    if ($case.Terminated) { $log += "`n" }
    [IO.File]::WriteAllText((Join-Path $dir 'mixed-1mb.stages.log'), $log, [Text.UTF8Encoding]::new($false))
    try {
        $rejected = $false
        try { & (Join-Path $PSScriptRoot '../extract-phase8-stages.ps1') -Directory $dir -Subsystem extension -ExpectedStages @('validation') | Out-Null } catch { $rejected = $true; $rejection = $_.Exception.Message }
        if ($case.Reject) {
            if (-not $rejected) { throw 'Malformed completion accepted' }
        } else {
            if ($rejected) { throw ('Valid interrupted/completed transcript rejected: ' + $rejection) }
            $rows = @(Import-Csv -LiteralPath (Join-Path $dir 'extension-stages.csv'))
            $summary = @(Import-Csv -LiteralPath (Join-Path $dir 'extension-summary.csv'))
            if ($rows.Count -ne $case.Rows -or $summary.Count -ne $case.Rows) { throw 'Censored/invalid completion became an observation' }
            $censorPath = Join-Path $dir 'extension-censored.csv'
            if ($null -ne $case.Censored) {
                $censored = @(Import-Csv -LiteralPath $censorPath)
                if ($censored.Count -ne 1 -or $censored[0].stage -ne $case.Censored) { throw 'Pending censored stage was lost' }
            } elseif (Test-Path -LiteralPath $censorPath) { throw 'Valid completion marked censored' }
        }
        'PASS: ' + $case.Name
    } catch {
        $failures += $case.Name + ': ' + $_.Exception.Message
        'FAIL: ' + $failures[-1]
    }
}
if ($failures.Count) { throw ($failures -join "`n") }
'Phase 8 stage regressions passed; evidence: ' + $regressionRoot
