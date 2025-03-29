#compdef kubetui

__kubetui_debug() {
    local file="$KUBETUI_COMP_DEBUG_FILE"
    if [[ -n ${file} ]]; then
        echo "$*" >> "${file}"
    fi
}

_kubetui() {
    typeset -a _arguments_options

    if is-at-least 5.2; then
        _arguments_options=(-S)
    else
        _arguments_options=()
    fi

    __kubetui_debug "\n========= starting completion logic =========="
    __kubetui_debug "CURRENT: ${CURRENT}, words[*]: ${words[*]}"

    local options=(
        '(-s --split-direction)'{-s+,--split-direction=}'[Window split direction]:v|h:((v\:"Vertical" h\:"Horizontal"))'
        '(-c --context)'{-c+,--context=}'[Context]:CONTEXT:__get_kubernetes_contexts'
        '(-A --all-namespaces -n --namespaces)'{-A,--all-namespaces=-}'[Select all namespaces]:true|false:(true false)'
        '(-C --kubeconfig)'{-C+,--kubeconfig=}'[kubeconfig path]:KUBECONFIG:_files'
        '(-l --logging)'{-l,--logging}'[Logging]'
        '(-h --help)'{-h,--help}'[Print help]'
        '(-V --version)'{-V,--version}'[Print version]'
        '(-A --all-namespaces)'\*{-n+,--namespaces=}'[Namespaces (e.g. -n val1,val2,val3 | -n val1 -n val2 -n val3)]:NAMESPACES:_sequence __get_kubernetes_namespaces'
        '--config-file=[Config file path]:CONFIG_FILE:_files'
    )

    _arguments "${_arguments_options[@]}" $options
}

(( $+functions[__complete_command] )) ||
__complete_command(){
    local truncated_words=("${=words[1,CURRENT]}")
    __kubetui_debug "Truncated words[*]: ${truncated_words[*]},"

    local last_param="${truncated_words[-1]}"
    local last_char="${last_param[-1]}"
    __kubetui_debug "last_param: ${last_param}, last_char: ${last_char}"

    # `kubetui __complete <command> -- <args>`
    local cmd="${truncated_words[1]} __complete $1 -- ${truncated_words[*]}"

    if [ "${last_char}" = "" ]; then
        cmd="${cmd} \"\""
    fi

    __kubetui_debug "About to call: eval ${cmd}"

    echo $cmd
}

(( $+functions[__get_kubernetes_resources] )) ||
__get_kubernetes_resources() {
    local type=$1;

    __kubetui_debug "Getting ${type}s..."

    local cmd=$(__complete_command "${type}")

    local result=("${(@f)$(eval $cmd 2>/dev/null)}")

    __kubetui_debug "${type}s: ${result[*]:-none}"

    _describe -t "${type}s" "${type}s" result
}

(( $+functions[__get_kubernetes_namespaces] )) ||
__get_kubernetes_namespaces() {
    __get_kubernetes_resources "namespace"
}

(( $+functions[__get_kubernetes_contexts] )) ||
__get_kubernetes_contexts() {
    __get_kubernetes_resources "context"
}

if [ "$funcstack[1]" = "_kubetui" ]; then
    _kubetui "$@"
else
    compdef _kubetui kubetui
fi
