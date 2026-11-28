#Requires -Version 7.0
$ErrorActionPreference = 'Stop'
$regressionRepo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$regressionRoot = Join-Path $regressionRepo ('target/extension-trace-regressions-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $regressionRoot -Force | Out-Null
$outer = 'begin,mixed-1mb,sample,0,load,scan'
$prefix = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,17', 'extension-begin,1,source_text')
$parseOpen = @('begin,mixed-1mb,sample,0,load,parse','parse-begin,1,source_text','parse-end,1,source_text,1','parse-begin,1,parser_init','parse-end,1,parser_init,2','parse-begin,1,decode_events','parse-end,1,decode_events,3','parse-begin,1,build_tree')
$parseComplete = $parseOpen + 'parse-end,1,build_tree,4'
$cases = @(
    @{ Name = 'torn-empty-duration'; Lines = $prefix + 'extension-end,1,source_text,'; Terminated = $true; Status = 'timeout'; Rows = 1; Censored = 'source_text'; Stages = @('validation','source_text') },
    @{ Name = 'torn-numeric-without-newline'; Lines = $prefix + 'extension-end,1,source_text,123'; Terminated = $false; Status = 'timeout'; Rows = 1; Censored = 'source_text'; Stages = @('validation','source_text') },
    @{ Name = 'censored-first-stage'; Lines = @($outer, 'extension-begin,1,validation'); Terminated = $true; Status = 'timeout'; Rows = 0; Censored = 'validation' },
    @{ Name = 'valid-zero-duration'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,0'); Terminated = $true; Status = 'completed'; Rows = 1; Censored = $null },
    @{ Name = 'reject-missing-warmup'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,17'); Terminated = $true; Status = 'completed'; Reject = $true },
    @{ Name = 'reject-negative-duration'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,-1'); Terminated = $true; Status = 'completed'; Reject = $true },
    @{ Name = 'reject-empty-completed-duration'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,1,validation,'); Terminated = $true; Status = 'completed'; Reject = $true },
    @{ Name = 'reject-mismatched-completion'; Lines = @($outer, 'extension-begin,1,validation', 'extension-end,99,validation,12'); Terminated = $true; Status = 'timeout'; Reject = $true }
    @{ Name = 'reject-completed-warmup-prefix-in-timeout'; Lines = @('begin,mixed-1mb,warmup,0,load,scan','extension-begin,1,validation','extension-end,1,validation,17','begin,mixed-1mb,sample,0,load,scan','extension-begin,2,validation'); Terminated=$true;Status='timeout';Reject=$true;CompletedKind='warmup';Stages=@('validation','source_text') },
    @{ Name = 'reject-open-wrong-first-stage'; Lines=@($outer,'extension-begin,1,source_text','extension-end,1,source_text,17','extension-begin,2,unrelated');Terminated=$true;Status='timeout';Reject=$true;Stages=@('validation','source_text') },
    @{ Name = 'reject-open-changed-call'; Lines=@($outer,'extension-begin,1,validation','extension-end,1,validation,17','extension-begin,2,source_text');Terminated=$true;Status='timeout';Reject=$true;Stages=@('validation','source_text') },
    @{ Name = 'torn-begin-before-stage'; Lines=@($outer,'extension-begin,1,validation','extension-end,1,validation,17','extension-begin,');Terminated=$false;Status='timeout';Rows=1;Censored=$null;UnknownTail=$true;Stages=@('validation','source_text') },
    @{ Name = 'torn-begin-within-stage'; Lines=@($outer,'extension-begin,1,validation','extension-end,1,validation,17','extension-begin,1,sour');Terminated=$false;Status='timeout';Rows=1;Censored=$null;UnknownTail=$true;Stages=@('validation','source_text') },
    @{ Name = 'torn-first-begin-prefix'; Lines=@($outer,'extension-be');Terminated=$false;Status='timeout';Rows=0;Censored=$null;UnknownTail=$true }
    @{ Name='parse-preset-complete';Subsystem='parse';DefaultPreset=$true;Lines=@('begin,mixed-1mb,sample,0,load,parse','parse-begin,1,source_text','parse-end,1,source_text,1','parse-begin,1,parser_init','parse-end,1,parser_init,2','parse-begin,1,decode_events','parse-end,1,decode_events,3','parse-begin,1,build_tree','parse-end,1,build_tree,4');Terminated=$true;Status='completed';Rows=4;Censored=$null },
    @{ Name='parse-preset-reject-missing';Subsystem='parse';DefaultPreset=$true;Lines=@('begin,mixed-1mb,sample,0,load,parse','parse-begin,1,source_text','parse-end,1,source_text,1','parse-begin,1,build_tree');Terminated=$true;Status='completed';Reject=$true },
    @{ Name='parse-torn-other-subsystem';Subsystem='parse';DefaultPreset=$true;Lines=$parseComplete+@('begin,mixed-1mb,sample,0,load,projection','p');Terminated=$false;Status='timeout';CompletedKind='sample';Rows=4;Censored=$null;ContextTail=$true },
    @{ Name='parse-torn-stats-prefix';Subsystem='parse';DefaultPreset=$true;Lines=$parseComplete+'parse-';Terminated=$false;Status='timeout';CompletedKind='sample';Rows=4;Censored=$null;ContextTail=$true },
    @{ Name='parse-torn-pending-end';Subsystem='parse';DefaultPreset=$true;Lines=$parseOpen+'parse-';Terminated=$false;Status='timeout';Rows=3;Censored='build_tree';ContextTail=$true },
    @{ Name='reject-parse-extra-begin';Subsystem='parse';DefaultPreset=$true;Lines=$parseComplete+'parse-begin,1,source_text';Terminated=$false;Status='timeout';CompletedKind='sample';Reject=$true },
    @{ Name='reject-parse-nested-begin-prefix';Subsystem='parse';DefaultPreset=$true;Lines=$parseOpen+'parse-beg';Terminated=$false;Status='timeout';Reject=$true }
)
$failures = @()
foreach ($case in $cases) {
    $subsystem = if ($case.Subsystem) { $case.Subsystem } else { 'extension' }
    $timingStage = if ($subsystem -eq 'parse') { 'parse' } else { 'scan' }
    $dir = Join-Path $regressionRoot $case.Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    [pscustomobject]@{ fixture = 'mixed-1mb'; status = $case.Status } | Export-Csv -LiteralPath (Join-Path $dir 'processes.csv') -NoTypeInformation -Encoding utf8
    $outerRows = @()
    if ($case.Status -eq 'completed') { $outerRows += [pscustomobject]@{fixture='mixed-1mb';kind='sample';sample=0;phase='load';stage=$timingStage;ns=1;items=0} }
    if ($case.Name -eq 'reject-missing-warmup') { $outerRows += [pscustomobject]@{fixture='mixed-1mb';kind='warmup';sample=0;phase='load';stage='scan';ns=1;items=0} }
    if ($case.CompletedKind) { $outerRows += [pscustomobject]@{fixture='mixed-1mb';kind=$case.CompletedKind;sample=0;phase='load';stage=$timingStage;ns=1;items=0} }
    if ($outerRows.Count) { $outerRows | Export-Csv -LiteralPath (Join-Path $dir 'mixed-1mb.csv') -NoTypeInformation -Encoding utf8 } else { 'fixture,kind,sample,phase,stage,ns,items' | Set-Content -LiteralPath (Join-Path $dir 'mixed-1mb.csv') -Encoding utf8 }
    $log = $case.Lines -join "`n"
    if ($case.Terminated) { $log += "`n" }
    [IO.File]::WriteAllText((Join-Path $dir 'mixed-1mb.stages.log'), $log, [Text.UTF8Encoding]::new($false))
    try {
        $rejected = $false
        $expectedStages = if ($case.Stages) { $case.Stages } else { @('validation') }
        $extractArgs = @{Directory=$dir;Subsystem=$subsystem}
        if (-not $case.DefaultPreset) { $extractArgs.ExpectedStages=$expectedStages }
        try { $result = & (Join-Path $PSScriptRoot '../extract-phase8-stages.ps1') @extractArgs | ConvertFrom-Json } catch { $rejected = $true; $rejection = $_.Exception.Message }
        if ($case.Reject) {
            if (-not $rejected) { throw 'Malformed completion accepted' }
        } else {
            if ($rejected) { throw ('Valid interrupted/completed transcript rejected: ' + $rejection) }
            $rows = @(Import-Csv -LiteralPath (Join-Path $dir ($subsystem+'-stages.csv')))
            $summary = @(Import-Csv -LiteralPath (Join-Path $dir ($subsystem+'-summary.csv')))
            if ($rows.Count -ne $case.Rows -or $summary.Count -ne $case.Rows) { throw 'Censored/invalid completion became an observation' }
            $censorPath = Join-Path $dir ($subsystem+'-censored.csv')
            if ($null -ne $case.Censored) {
                $censored = @(Import-Csv -LiteralPath $censorPath)
                if ($censored.Count -ne 1 -or $censored[0].stage -ne $case.Censored) { throw 'Pending censored stage was lost' }
            } elseif (Test-Path -LiteralPath $censorPath) { throw 'Valid completion marked censored' }
            if ($case.UnknownTail -and $result.unidentified_begin_tails -ne 1) { throw 'Unidentified interrupted marker evidence lost' }
            if ($case.ContextTail -and $result.unidentified_begin_tails -ne 0) { throw 'Context-resolved tail invented a begin' }
        }
        'PASS: ' + $case.Name
    } catch {
        $failures += $case.Name + ': ' + $_.Exception.Message
        'FAIL: ' + $failures[-1]
    }
}
if ($failures.Count) { throw ($failures -join "`n") }
'Phase 8 stage regressions passed; evidence: ' + $regressionRoot
