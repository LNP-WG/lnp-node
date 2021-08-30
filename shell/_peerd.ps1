
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'peerd' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'peerd'
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
        'peerd' {
            [CompletionResult]::new('-L', 'L', [CompletionResultType]::ParameterName, 'Start daemon in listening mode binding the provided local address')
            [CompletionResult]::new('--listen', 'listen', [CompletionResultType]::ParameterName, 'Start daemon in listening mode binding the provided local address')
            [CompletionResult]::new('-C', 'C', [CompletionResultType]::ParameterName, 'Connect to a remote peer with the provided address after start')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'Connect to a remote peer with the provided address after start')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Customize port used by lightning peer network')
            [CompletionResult]::new('--port', 'port', [CompletionResultType]::ParameterName, 'Customize port used by lightning peer network')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'Overlay peer communications through different transport protocol')
            [CompletionResult]::new('--overlay', 'overlay', [CompletionResultType]::ParameterName, 'Overlay peer communications through different transport protocol')
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'Node key file')
            [CompletionResult]::new('--key-file', 'key-file', [CompletionResultType]::ParameterName, 'Node key file')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'RGB configuration: ignored by this daemon')
            [CompletionResult]::new('--rgb20-rpc', 'rgb20-rpc', [CompletionResultType]::ParameterName, 'RGB configuration: ignored by this daemon')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Data directory path')
            [CompletionResult]::new('--data-dir', 'data-dir', [CompletionResultType]::ParameterName, 'Data directory path')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Path to the configuration file')
            [CompletionResult]::new('--config', 'config', [CompletionResultType]::ParameterName, 'Path to the configuration file')
            [CompletionResult]::new('-T', 'T', [CompletionResultType]::ParameterName, 'Use Tor')
            [CompletionResult]::new('--tor-proxy', 'tor-proxy', [CompletionResultType]::ParameterName, 'Use Tor')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'ZMQ socket name/address to forward all incoming lightning messages')
            [CompletionResult]::new('--msg-socket', 'msg-socket', [CompletionResultType]::ParameterName, 'ZMQ socket name/address to forward all incoming lightning messages')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for daemon control interface')
            [CompletionResult]::new('--ctl-socket', 'ctl-socket', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for daemon control interface')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'Blockchain to use')
            [CompletionResult]::new('--chain', 'chain', [CompletionResultType]::ParameterName, 'Blockchain to use')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
