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
		"--pod-columns"
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
	--pod-columns)
		__kubetui_pod_columns "${cur}"
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

__kubetui_pod_columns() {
	local cur="$1"

	local all_values=(name ready status restarts age ip node nominatednode readinessgates full)

	local old_ifs="$IFS"
	IFS=',' read -ra used <<<"$cur"
	IFS="$old_ifs"

	local last=""
	if [[ "$cur" == *, ]]; then
		last=""
	elif [[ ${#used[@]} -gt 0 ]]; then
		last="${used[${#used[@]} - 1]}"
	fi

	for u in "${used[@]}"; do
		[[ "$u" == "full" ]] && return 0
	done

	# すでに何か指定済みの場合、候補から full を除外
	if [[ ${#used[@]} -gt 1 || "$cur" == *,* ]]; then
		local filtered=()
		for val in "${all_values[@]}"; do
			[[ "$val" == "full" ]] && continue
			filtered+=("$val")
		done
		all_values=("${filtered[@]}")
	fi

	local candidates=()
	for val in "${all_values[@]}"; do
		local found=false
		for u in "${used[@]}"; do
			[[ "$u" == "$val" ]] && {
				found=true
				break
			}
		done
		! $found && candidates+=("$val")
	done

	COMPREPLY=($(compgen -W "${candidates[*]}" -- "$last"))

	if [[ ${#used[@]} -gt 1 || "$cur" == *, ]]; then
		__kubetui_debug "Used values: ${used[*]}, length: ${#used[@]}"
		local -a prefix_parts
		local last="${used[-1]}"
		local exact_match=false
		for v in "${all_values[@]}"; do
			if [[ "$v" == "$last" ]]; then
				exact_match=true
				break
			fi
		done

		if $exact_match; then
			prefix_parts=("${used[@]}")
			last=""
		else
			prefix_parts=("${used[@]:0:${#used[@]}-1}")
			last="${used[-1]}"
		fi

		local old_ifs="$IFS"
		IFS=,
		local prefix="${prefix_parts[*]}"
		IFS="$old_ifs"

		__kubetui_debug "Prefix: '${prefix}'"
		for i in "${!COMPREPLY[@]}"; do
			if [[ -n "$prefix" ]]; then
				COMPREPLY[$i]="${prefix},${COMPREPLY[$i]}"
			fi
		done

		__kubetui_debug "Updated COMPREPLY: ${COMPREPLY[*]}"
	fi

}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
	complete -F _kubetui -o nosort kubetui
else
	complete -F _kubetui kubetui
fi

# vim: ts=4 sw=4 sts=4 noet filetype=sh
