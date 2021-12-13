
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'signd' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'signd'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-')) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'signd' {
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Data directory path')
            [CompletionResult]::new('--data-dir', 'data-dir', [CompletionResultType]::ParameterName, 'Data directory path')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Path for the configuration file')
            [CompletionResult]::new('--config', 'config', [CompletionResultType]::ParameterName, 'Path for the configuration file')
            [CompletionResult]::new('-T', 'T', [CompletionResultType]::ParameterName, 'Use Tor')
            [CompletionResult]::new('--tor-proxy', 'tor-proxy', [CompletionResultType]::ParameterName, 'Use Tor')
            [CompletionResult]::new('--msg', 'msg', [CompletionResultType]::ParameterName, 'ZMQ socket for internal message bus')
            [CompletionResult]::new('--ctl', 'ctl', [CompletionResultType]::ParameterName, 'ZMQ socket for internal service bus')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--rpc', 'rpc', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'Blockchain to use')
            [CompletionResult]::new('--chain', 'chain', [CompletionResultType]::ParameterName, 'Blockchain to use')
            [CompletionResult]::new('--electrum-server', 'electrum-server', [CompletionResultType]::ParameterName, 'Electrum server to use')
            [CompletionResult]::new('--electrum-port', 'electrum-port', [CompletionResultType]::ParameterName, 'Customize Electrum server port number. By default the wallet will use port matching the selected network')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--threaded-daemons', 'threaded-daemons', [CompletionResultType]::ParameterName, 'Spawn daemons as threads and not processes')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
