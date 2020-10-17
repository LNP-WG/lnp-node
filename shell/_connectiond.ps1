
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'connectiond' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'connectiond'
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
        'connectiond' {
            [CompletionResult]::new('-L', 'L', [CompletionResultType]::ParameterName, 'Start daemon in listening mode binding the provided local address')
            [CompletionResult]::new('--listen', 'listen', [CompletionResultType]::ParameterName, 'Start daemon in listening mode binding the provided local address')
            [CompletionResult]::new('-C', 'C', [CompletionResultType]::ParameterName, 'Connect to a remote peer with the provided address after start')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'Connect to a remote peer with the provided address after start')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Customize port used by lightning peer network')
            [CompletionResult]::new('--port', 'port', [CompletionResultType]::ParameterName, 'Customize port used by lightning peer network')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Spawn threads instead of forking new processes for incoming connections')
            [CompletionResult]::new('--use-threads', 'use-threads', [CompletionResultType]::ParameterName, 'Spawn threads instead of forking new processes for incoming connections')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Data directory path')
            [CompletionResult]::new('--data-dir', 'data-dir', [CompletionResultType]::ParameterName, 'Data directory path')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Path to the configuration file')
            [CompletionResult]::new('--config', 'config', [CompletionResultType]::ParameterName, 'Path to the configuration file')
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'Node key file')
            [CompletionResult]::new('--key-file', 'key-file', [CompletionResultType]::ParameterName, 'Node key file')
            [CompletionResult]::new('-T', 'T', [CompletionResultType]::ParameterName, 'Use Tor')
            [CompletionResult]::new('--tor-proxy', 'tor-proxy', [CompletionResultType]::ParameterName, 'Use Tor')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'ZMQ socket name/address to forward all incoming lightning messages')
            [CompletionResult]::new('--msg-socket', 'msg-socket', [CompletionResultType]::ParameterName, 'ZMQ socket name/address to forward all incoming lightning messages')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for daemon control interface')
            [CompletionResult]::new('--ctl-socket', 'ctl-socket', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for daemon control interface')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Prints help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Prints help information')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Prints version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Prints version information')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
