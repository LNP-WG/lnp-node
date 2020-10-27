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
            
            help)
                cmd+="__help"
                ;;
            init)
                cmd+="__init"
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
            opts=" -d -c -v -T -m -x -n -h -V  --data-dir --config --verbose --tor-proxy --msg-socket --ctl-socket --chain --help --version  init ping help"
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
        lnp__cli__init)
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
        lnp__cli__ping)
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
    esac
}

complete -F _lnp-cli -o bashdefault -o default lnp-cli
