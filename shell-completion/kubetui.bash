__kubetui_debug() {
	local file="$KUBETUI_COMP_DEBUG_FILE"

	if [[ -n ${file} ]]; then
		echo "$*" >>"${file}"
	fi
}

__kubetui_options() {
	local options=(
		"-h" "--help"
		"-V" "--version"
		"-A" "--all-namespaces"
		"-c" "--context"
		"-C" "--kubeconfig"
		"--config-file"
		"-l" "--logging"
		"-n" "--namespaces"
		"-s" "--split-direction"
	)

	echo "${options[*]}"
}

_kubetui() {
	__kubetui_debug $'\n''========= starting completion logic =========='

	local cur="${COMP_WORDS[COMP_CWORD]}"
	local prev="${COMP_WORDS[COMP_CWORD - 1]}"
	local cword=${COMP_CWORD}
	local words=("${COMP_WORDS[@]}")

	__kubetui_debug "COMP_CWORD: ${COMP_CWORD}, COMP_WORDS[*]: '${COMP_WORDS[*]}'"
	__kubetui_debug "cur: ${cur}, prev: ${prev}, words[*]: '${words[*]}', #words: ${#words[*]}"

	local options=$(__kubetui_options)

	case "${prev}" in
	-s | --split-direction)
		COMPREPLY=($(compgen -W "v h" -- "${cur}"))
		return 0
		;;
	-n | --namespaces)
		COMPREPLY=($(compgen -W "$(__kubetui_get_kubernetes_namespaces)" -- "${cur}"))
		return 0
		;;
	-c | --context)
		COMPREPLY=($(compgen -W "$(__kubetui_get_kubernetes_contexts)" -- "${cur}"))
		return 0
		;;
	-C | --kubeconfig | --config-file)
		COMPREPLY=($(compgen -f "${cur}"))
		return 0
		;;
	esac

	COMPREPLY=($(compgen -W "${options[*]}" -- "${cur}"))

	return 0
}

__kubetui_complete_command() {
	local truncated_words=("${words[@]:0:$cword+1}")
	__kubetui_debug "Truncated words[*]: '${truncated_words[*]}'"

	local last_param="${truncated_words[@]: -1}"
	local last_char="${last_param[@]: -1}"
	__kubetui_debug "last_param: '${last_param}', last_char: '${last_char}'"

	# `kubetui __complete <command> -- <args>`
	local cmd="${truncated_words[0]} __complete $1 -- ${truncated_words[*]}"

	if [ "${last_char}" = "" ]; then
		cmd="${cmd} \"\""
	fi

	__kubetui_debug "About to call: eval ${cmd}"

	echo $cmd
}

__kubetui_get_kubernetes_resources() {
	local type=$1

	__kubetui_debug "Getting ${type}s..."

	local cmd=$(__kubetui_complete_command "${type}")

	local result="$(eval $cmd 2>/dev/null)"

	__kubetui_debug "${type}s: ${result[@]:-none}"

	echo "${result[*]}"
}

__kubetui_get_kubernetes_namespaces() {
	__kubetui_get_kubernetes_resources "namespace"
}

__kubetui_get_kubernetes_contexts() {
	__kubetui_get_kubernetes_resources "context"
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
	complete -F _kubetui -o nosort kubetui
else
	complete -F _kubetui kubetui
fi

# vim: ts=4 sw=4 sts=4 noet filetype=sh
