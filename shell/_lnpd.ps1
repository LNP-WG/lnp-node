
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'lnpd' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'lnpd'
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
        'lnpd' {
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for RGB Node fungible RPC interface (RGB20 RPC)')
            [CompletionResult]::new('--rgb20-rpc', 'rgb20-rpc', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for RGB Node fungible RPC interface (RGB20 RPC)')
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'Node key file')
            [CompletionResult]::new('--key-file', 'key-file', [CompletionResultType]::ParameterName, 'Node key file')
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
            [CompletionResult]::new('--electrum-server', 'electrum-server', [CompletionResultType]::ParameterName, 'Electrum server to use. `extract` command')
            [CompletionResult]::new('--electrum-port', 'electrum-port', [CompletionResultType]::ParameterName, 'Customize Electrum server port number. By default the wallet will use port matching the selected network')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--threaded', 'threaded', [CompletionResultType]::ParameterName, 'Spawn LNP services as threads instead of processes')
            [CompletionResult]::new('--threaded-daemons', 'threaded-daemons', [CompletionResultType]::ParameterName, 'Spawn daemons as threads and not processes')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize data directory')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'lnpd;init' {
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for RGB Node fungible RPC interface (RGB20 RPC)')
            [CompletionResult]::new('--rgb20-rpc', 'rgb20-rpc', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for RGB Node fungible RPC interface (RGB20 RPC)')
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
            [CompletionResult]::new('--electrum-server', 'electrum-server', [CompletionResultType]::ParameterName, 'Electrum server to use. `extract` command')
            [CompletionResult]::new('--electrum-port', 'electrum-port', [CompletionResultType]::ParameterName, 'Customize Electrum server port number. By default the wallet will use port matching the selected network')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnpd;help' {
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for RGB Node fungible RPC interface (RGB20 RPC)')
            [CompletionResult]::new('--rgb20-rpc', 'rgb20-rpc', [CompletionResultType]::ParameterName, 'ZMQ socket name/address for RGB Node fungible RPC interface (RGB20 RPC)')
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
            [CompletionResult]::new('--electrum-server', 'electrum-server', [CompletionResultType]::ParameterName, 'Electrum server to use. `extract` command')
            [CompletionResult]::new('--electrum-port', 'electrum-port', [CompletionResultType]::ParameterName, 'Customize Electrum server port number. By default the wallet will use port matching the selected network')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
