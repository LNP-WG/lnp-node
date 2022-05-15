
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'lnp-cli' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'lnp-cli'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'lnp-cli' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-V', 'V', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('listen', 'listen', [CompletionResultType]::ParameterValue, 'Bind to a socket and start listening for incoming LN peer connections')
            [CompletionResult]::new('connect', 'connect', [CompletionResultType]::ParameterValue, 'Connect to the remote lightning network peer')
            [CompletionResult]::new('ping', 'ping', [CompletionResultType]::ParameterValue, 'Ping remote peer (must be already connected)')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'General information about the running node')
            [CompletionResult]::new('funds', 'funds', [CompletionResultType]::ParameterValue, 'Lists all funds available for channel creation with the list of assets and provides information about funding points (bitcoin address or UTXO for RGB assets)')
            [CompletionResult]::new('peers', 'peers', [CompletionResultType]::ParameterValue, 'Lists existing peer connections')
            [CompletionResult]::new('channels', 'channels', [CompletionResultType]::ParameterValue, 'Lists existing channels')
            [CompletionResult]::new('open', 'open', [CompletionResultType]::ParameterValue, 'Opens a new channel with a remote peer, which must be already connected')
            [CompletionResult]::new('invoice', 'invoice', [CompletionResultType]::ParameterValue, 'Create an invoice')
            [CompletionResult]::new('pay', 'pay', [CompletionResultType]::ParameterValue, 'Pay the invoice')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'lnp-cli;listen' {
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'IPv4 or IPv6 address to bind to')
            [CompletionResult]::new('--ip', 'ip', [CompletionResultType]::ParameterName, 'IPv4 or IPv6 address to bind to')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Port to use; defaults to the native LN port')
            [CompletionResult]::new('--port', 'port', [CompletionResultType]::ParameterName, 'Port to use; defaults to the native LN port')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'Use overlay protocol (http, websocket etc)')
            [CompletionResult]::new('--overlay', 'overlay', [CompletionResultType]::ParameterName, 'Use overlay protocol (http, websocket etc)')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;connect' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;ping' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;info' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;funds' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;peers' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;channels' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;open' {
            [CompletionResult]::new('--pay', 'pay', [CompletionResultType]::ParameterName, 'Amount of millisatoshis to pay to the remote peer at channel opening')
            [CompletionResult]::new('--fee-rate', 'fee-rate', [CompletionResultType]::ParameterName, 'Sets fee rate for the channel transacitons')
            [CompletionResult]::new('--announce-channel', 'announce-channel', [CompletionResultType]::ParameterName, 'Make channel public and route payments')
            [CompletionResult]::new('--channel-type', 'channel-type', [CompletionResultType]::ParameterName, 'Channel type as defined in BOLT-2')
            [CompletionResult]::new('--dust-limit', 'dust-limit', [CompletionResultType]::ParameterName, 'The threshold below which outputs on transactions broadcast by sender will be omitted')
            [CompletionResult]::new('--to-self-delay', 'to-self-delay', [CompletionResultType]::ParameterName, 'The number of blocks which the counterparty will have to wait to claim on-chain funds if they broadcast a commitment transaction')
            [CompletionResult]::new('--htlc-max-count', 'htlc-max-count', [CompletionResultType]::ParameterName, 'The maximum number of the received HTLCs')
            [CompletionResult]::new('--htlc-min-value', 'htlc-min-value', [CompletionResultType]::ParameterName, 'Indicates the smallest value of an HTLC this node will accept, in milli-satoshi')
            [CompletionResult]::new('--htlc-max-total-value', 'htlc-max-total-value', [CompletionResultType]::ParameterName, 'The maximum inbound HTLC value in flight towards this node, in milli-satoshi')
            [CompletionResult]::new('--channel-reserve', 'channel-reserve', [CompletionResultType]::ParameterName, 'The minimum value unencumbered by HTLCs for the counterparty to keep in the channel, in satoshis')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;invoice' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;pay' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
        'lnp-cli;help' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('--connect', 'connect', [CompletionResultType]::ParameterName, 'ZMQ socket for connecting daemon RPC interface')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Set verbosity level')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Set verbosity level')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
