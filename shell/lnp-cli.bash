_lnp-cli() {
    local i cur prev opts cmds
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${i}" in
            lnp-cli)
                cmd="lnp__cli"
                ;;
            
            connect)
                cmd+="__connect"
                ;;
            create-channel)
                cmd+="__create__channel"
                ;;
            fund-channel)
                cmd+="__fund__channel"
                ;;
            funding-info)
                cmd+="__funding__info"
                ;;
            help)
                cmd+="__help"
                ;;
            invoice)
                cmd+="__invoice"
                ;;
            list-channels)
                cmd+="__list__channels"
                ;;
            list-funds)
                cmd+="__list__funds"
                ;;
            list-peers)
                cmd+="__list__peers"
                ;;
            listen)
                cmd+="__listen"
                ;;
            pay)
                cmd+="__pay"
                ;;
            ping)
                cmd+="__ping"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        lnp__cli)
            opts=" -d -c -v -T -m -x -n -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --chain --help --version  listen connect ping funding-info list-funds list-peers list-channels create-channel fund-channel invoice pay help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --chain)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -n)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        
        lnp__cli__connect)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  <peer> "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__create__channel)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  <peer> <satoshis> "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__fund__channel)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  <channel> <asset>... "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__funding__info)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__help)
            opts=" -h -V  --help --version  "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__invoice)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  <amount> <asset> "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__list__channels)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__list__funds)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  <asset>... "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__list__peers)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__listen)
            opts=" -i -p -o -d -c -v -T -m -x -h -V  --ip --port --overlay --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --ip)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -i)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --port)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -p)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --overlay)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__pay)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  <invoice> <channel> "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        lnp__cli__ping)
            opts=" -d -c -v -T -m -x -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --help --version  <peer> "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --data-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -d)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tor-proxy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --msg-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ctl-socket)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -x)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

complete -F _lnp-cli -o bashdefault -o default lnp-cli
