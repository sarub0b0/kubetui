#compdef kubetui

autoload -Uz is-at-least

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
        '(-s --split-direction)'{-s,--split-direction}'[Window split direction]:v|h:((v\:"Vertical" h\:"Horizontal"))'
        '(-c --context)'{-c,--context}'[Context]:CONTEXT:__kubetui_get_kubernetes_contexts'
        '(-A --all-namespaces -n --namespaces)'{-A,--all-namespaces=-}'[Select all namespaces]:true|false:(true false)'
        '(-C --kubeconfig)'{-C,--kubeconfig}'[kubeconfig path]:KUBECONFIG:_files'
        '(-l --logging)'{-l,--logging}'[Logging]'
        '(-h --help)'{-h,--help}'[Print help]'
        '(-V --version)'{-V,--version}'[Print version]'
        '(-A --all-namespaces)'\*{-n,--namespaces}'[Namespaces (e.g. -n val1,val2,val3 | -n val1 -n val2 -n val3)]:NAMESPACES:_sequence __kubetui_get_kubernetes_namespaces'
        '--pod-columns[Comma-separated list of columns to show in pod table (e.g. name,status,ip). Use "full" to show all available columns.]:POD_COLUMNS:_sequence __kubetui_pod_columns'
        '--config-file[Config file path]:CONFIG_FILE:_files'
    )

    _arguments "${_arguments_options[@]}" $options
}

(( $+functions[__kubetui_complete_command] )) ||
__kubetui_complete_command(){
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

(( $+functions[__kubetui_get_kubernetes_resources] )) ||
__kubetui_get_kubernetes_resources() {
    local type=$1;

    __kubetui_debug "Getting ${type}s..."

    local cmd=$(__kubetui_complete_command "${type}")

    local result=("${(@f)$(eval $cmd 2>/dev/null)}")

    __kubetui_debug "${type}s: ${result[*]:-none}"

    if [[ -n "${result[@]}" ]]; then
        _describe -t "${type}s" "${type}s" result
    else
        _message -e "${type}s" "no ${type}s found"
    fi
}

(( $+functions[__kubetui_get_kubernetes_namespaces] )) ||
__kubetui_get_kubernetes_namespaces() {
    __kubetui_get_kubernetes_resources "namespace"
}

(( $+functions[__kubetui_get_kubernetes_contexts] )) ||
__kubetui_get_kubernetes_contexts() {
    __kubetui_get_kubernetes_resources "context"
}

(( $+functions[__kubetui_pod_columns] )) ||
__kubetui_pod_columns() {
    local pod_columns_values=(
        name
        ready
        status
        restarts
        age
        ip
        node
        nominatednode
        readinessgates
    )

    ## 入力済み値を取得（カンマで分割）
    local cur="${words[CURRENT]##*,}"
    local used=("${(s:,:)words[CURRENT]}")
    local last_param="${used[-1]}"

    __kubetui_debug "Current value: ${cur}"
    __kubetui_debug "Used values: ${used[*]}, length: ${#used[@]}"
    __kubetui_debug "Last parameter: ${last_param}"

    # usedの要素数が0のときpod_columns_valuesにfullを追加
    if [[ ${#used[@]} -eq 1 ]]; then
        pod_columns_values+=("full")
    fi

    __kubetui_debug "Pod columns values: ${pod_columns_values[*]}"

    # `full` が入っていたら補完を無効化
    if [[ "${used[*]}" =~ "full" ]]; then
        _message -e 'pod columns' "The 'full' option is already selected, no further columns can be added."
        return
    fi

    # 候補にない値が入力さている場合は、処理を停止
    if [[ -n "${cur}" && ! "${pod_columns_values[*]}" =~ "${cur}" ]]; then
        _message -e 'pod columns' "Invalid pod column: '${cur}'."
        return
    fi

    ## 補完候補リストから used を除外
    local -a filtered_values=()
    for val in "${pod_columns_values[@]}"; do
        local is_used=false

        for used_val in "${used[@]}"; do
            if [[ "${val}" == "${used_val}" ]] && [[ "$last_param" != "${used_val}" ]]; then
                is_used=true
                break
            fi
        done

        if [[ "${is_used}" == false ]]; then
            filtered_values+=("${val}")
        fi
    done

    __kubetui_debug "Filtered values: ${filtered_values[*]}"

    if [[ -n "${filtered_values[*]}" ]]; then
        _describe -t 'pod columns' 'pod columns' filtered_values
    else
        _message -e 'pod columns' "No more pod columns available to add."
    fi
}

if [ "$funcstack[1]" = "_kubetui" ]; then
    _kubetui "$@"
else
    compdef _kubetui kubetui
fi
