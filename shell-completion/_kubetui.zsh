#compdef kubetui

__kubetui_debug() {
    local file="$KUBETUI_COMP_DEBUG_FILE"
    if [[ -n ${file} ]]; then
        echo "$*" >> "${file}"
    fi
}

_kubetui() {
    typeset -A opt_args
    typeset -a _arguments_options

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi


    local -a commands options
    local context curcontext="$curcontext" state line

    __kubetui_debug "\n========= starting completion logic =========="
    __kubetui_debug "CURRENT: ${CURRENT}, words[*]: ${words[*]}"

    commands=(
        "completion:Generate shell completion script"
    )

    options=(
        '(-s --split-direction)'{-s+,--split-direction=}'[Window split direction]:v|h:((v\:"Vertical" h\:"Horizontal"))'
        '(-c --context)'{-c+,--context=}'[Context]:CONTEXT:_get_kubernetes_contexts'
        '(-A --all-namespaces -n --namespaces)'{-A,--all-namespaces=-}'[Select all namespaces]:true|false:(true false)'
        '(-C --kubeconfig)'{-C+,--kubeconfig=}'[kubeconfig path]:KUBECONFIG:_files'
        '(-l --logging)'{-l,--logging}'[Logging]'
        '(-h --help)'{-h,--help}'[Print help]'
        '(-V --version)'{-V,--version}'[Print version]'
        '(-A --all-namespaces)'\*{-n+,--namespaces=}'[Namespaces (e.g. -n val1,val2,val3 | -n val1 -n val2 -n val3)]:NAMESPACES:_sequence _get_kubernetes_namespaces'
        '--config-file=[Config file path]:CONFIG_FILE:_files'
    )

    _arguments "${_arguments_options[@]}" $options
}

(( $+functions[_complete_command] )) ||
_complete_command(){
    local cmd last_param last_char;
    local -a truncated_words;

    truncated_words=("${=words[1,CURRENT]}")
    __kubetui_debug "Truncated words[*]: ${truncated_words[*]},"

    # `kubetui __complete <command> -- <args>`
    cmd="${truncated_words[1]} __complete $1 -- ${truncated_words[*]}"

    last_param="${truncated_words[-1]}"
    last_char="${last_param[-1]}"
    __kubetui_debug "last_param: ${last_param}, last_char: ${last_char}"

    if [ "${last_char}" = "" ]; then
        cmd="${cmd} \"\""
    fi

    __kubetui_debug "About to call: eval ${cmd}"
    echo $cmd
}

(( $+functions[_get_kubernetes_resources] )) ||
_get_kubernetes_resources() {
    local type=$1;
    local cmd result;

    __kubetui_debug "Getting ${type}s..."

    cmd=$(_complete_command "${type}")

    result=("${(@f)$(eval $cmd 2>/dev/null)}")

    __kubetui_debug "${type}s: ${result[*]}"

    _describe -t "${type}s" "${type}s" result
}

(( $+functions[_get_kubernetes_namespaces] )) ||
_get_kubernetes_namespaces() {
    _get_kubernetes_resources "namespace"
}

(( $+functions[_get_kubernetes_contexts] )) ||
_get_kubernetes_contexts() {
    _get_kubernetes_resources "context"
}

if [ "$funcstack[1]" = "_kubetui" ]; then
    _kubetui "$@"
else
    compdef _kubetui kubetui
fi
