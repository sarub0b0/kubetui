global playback_speed, typing_speed

set playback_speed to 1.0

set typing_speed to 0.05

on sec(s)
	return s * playback_speed
end sec

on delay_sec(s)
	delay sec(s)
end delay_sec

on delayed_keystroke(key)
	tell application "System Events"
		if class of key is string then
			repeat with c in key
				keystroke c
				my delay_sec(typing_speed)
			end repeat
		else
			keystroke key
			my delay_sec(typing_speed)
		end if
	end tell
end delayed_keystroke

on delayed_keycode(key)
	tell application "System Events"
		key code key
		my delay_sec(typing_speed)
	end tell
end delayed_keycode

on delayed_keystroke_ctrl_w()
	tell application "System Events"
		keystroke "w" using control down
		my delay_sec(typing_speed)
	end tell
end delayed_keystroke_ctrl_w

on delayed_keystroke_escape()
	tell application "System Events"
		-- keystroke "[" using control down
		key code 30 using control down
		my delay_sec(typing_speed)
	end tell
end delayed_keystroke_escape

on delayed_keystroke_backtab()
	tell application "System Events"
		keystroke tab using shift down
		my delay_sec(typing_speed)
	end tell
end delayed_keystroke_backtab

on open_kubetui()
	delayed_keystroke("kubetui")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
end open_kubetui

on close_kubetui()
	delayed_keystroke_escape()
	my delay_sec(1)
end close_kubetui

on pod_tab()
	my pod_tab_list()
	
	my pod_tab_list_filter()
	
	my pod_tab_open_yaml()
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	my pod_tab_log_query()
	
	my pod_tab_log()
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
end pod_tab

on pod_tab_list()
	delayed_keystroke(return)
	
	my delay_sec(1)
	
	delayed_keystroke("jjj")
	
	my delay_sec(1)
	
	delayed_keystroke(return)
	
	my delay_sec(1)
	
	delayed_keystroke("j")
	
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
end pod_tab_list

on pod_tab_list_filter()
	delayed_keystroke("/")
	my delay_sec(1)
	
	delayed_keystroke("app")
	my delay_sec(1)
	
	delayed_keycode(49)
	my delay_sec(1)
	
	delayed_keystroke("error")
	my delay_sec(1)
	
	delayed_keystroke_ctrl_w()
	my delay_sec(1)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end pod_tab_list_filter

on pod_tab_open_yaml()
	delayed_keystroke("y")
	my delay_sec(1)
	
	delayed_keystroke("jjjjj")
	my delay_sec(1)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end pod_tab_open_yaml

on pod_tab_log()
	my pod_tab_log_search()
	my pod_tab_log_insert_blankline(3)
end pod_tab_log

on pod_tab_log_insert_blankline(n)
	repeat n times
		delayed_keystroke(return)
		my delay_sec(0.1)
	end repeat
	my delay_sec(1)
end pod_tab_log_insert_blankline

on pod_tab_log_search()
	delayed_keystroke("/")
	my delay_sec(2)
	
	delayed_keystroke("docker")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	repeat 3 times
		delayed_keystroke("n")
		my delay_sec(1)
	end repeat
	
	delayed_keystroke_escape()
	my delay_sec(1)
end pod_tab_log_search

on pod_tab_log_query()
	delayed_keystroke_ctrl_w()
	my delay_sec(1)
	
	delayed_keystroke("pod")
	delayed_keycode(39)
	delayed_keystroke("app")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(2)
	
	delayed_keystroke_ctrl_w()
	my delay_sec(1)
	
	delayed_keystroke("deploy/app")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(2)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("g")
	my delay_sec(1)
end pod_tab_log_query

on activate_tab_by_id(id)
	my delayed_keystroke(id)
end activate_tab_by_id

on config_tab_open_yaml()
	delayed_keystroke("y")
	my delay_sec(1)
	
	delayed_keystroke("jjjjj")
	my delay_sec(1)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end config_tab_open_yaml

on config_tab()
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke("j")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke("j")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke("jjjj")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	config_tab_open_yaml()
	my delay_sec(1)
end config_tab

on network_tab_open_yaml()
	delayed_keystroke("y")
	my delay_sec(1)
	
	delayed_keystroke("jjjjj")
	my delay_sec(1)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end network_tab_open_yaml

on network_tab()
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("/")
	my delay_sec(1)
	
	delayed_keystroke("rela")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("jjj")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("G")
	my delay_sec(1)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("jj")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("G")
	my delay_sec(1)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("G")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("jjjj")
	my delay_sec(1)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	my network_tab_open_yaml()
	my delay_sec(1)
	
end network_tab

on event_tab()
	my delay_sec(3)
end event_tab

on list_tab()
	delayed_keystroke("f")
	my delay_sec(1)
	
	delayed_keystroke("pods")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke_ctrl_w()
	my delay_sec(1)
	
	delayed_keystroke("deploy")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end list_tab

on yaml_tab()
	delayed_keystroke("f")
	my delay_sec(1)
	
	delayed_keystroke("pods")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke("app")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(2)
	
	delayed_keystroke("jjjjj")
	my delay_sec(1)
	
	delayed_keystroke("/")
	my delay_sec(1)
	
	delayed_keystroke("image")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke("n")
	my delay_sec(1)
	
	delayed_keystroke("n")
	my delay_sec(1)
end yaml_tab

on select_namespaces()
	delayed_keystroke("n")
	my delay_sec(1)
	
	delayed_keystroke("sys")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(2)
	
	delayed_keystroke("N")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
	
	delayed_keystroke_escape()
	my delay_sec(1)
	
	delayed_keystroke("G")
	my delay_sec(1)
	
	my activate_tab_by_id("2")
	my delay_sec(2)
	
	my activate_tab_by_id("3")
	my delay_sec(2)
	
	my activate_tab_by_id("4")
	my delay_sec(2)
	
	my activate_tab_by_id("5")
	my delay_sec(2)
end select_namespaces

on select_context()
	delayed_keystroke("c")
	my delay_sec(1)
	
	delayed_keystroke("2")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(2)
	
	my activate_tab_by_id("2")
	my delay_sec(2)
	
	my activate_tab_by_id("3")
	my delay_sec(2)
	
	delayed_keystroke("c")
	my delay_sec(1)
	
	delayed_keystroke("kind")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(1)
end select_context

on help_popup()
	delayed_keystroke("h")
	my delay_sec(2)
	
	delayed_keystroke("jj")
	my delay_sec(1)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end help_popup

on demo()
	my delay_sec(1)
	
	my open_kubetui()
	my delay_sec(1)
	
	my pod_tab()
	my delay_sec(1)
	
	my activate_tab_by_id("2")
	my delay_sec(1)
	
	my config_tab()
	my delay_sec(1)
	
	my activate_tab_by_id("3")
	my delay_sec(1)
	
	my network_tab()
	my delay_sec(1)
	
	my activate_tab_by_id("4")
	my delay_sec(1)
	
	my event_tab()
	my delay_sec(1)
	
	my activate_tab_by_id("5")
	my delay_sec(1)
	
	my list_tab()
	my delay_sec(1)
	
	my activate_tab_by_id("6")
	my delay_sec(1)
	
	my yaml_tab()
	my delay_sec(1)
	
	my activate_tab_by_id("1")
	my delay_sec(1)
	
	my select_namespaces()
	my delay_sec(1)
	
	my activate_tab_by_id("1")
	my delay_sec(1)
	
	my select_context()
	my delay_sec(1)
	
	my activate_tab_by_id("1")
	my delay_sec(1)
	
	my help_popup()
	my delay_sec(1)
	
	my close_kubetui()
end demo

tell application "iTerm"
	activate
	tell current window
		tell current session of current tab
			my demo()
		end tell
	end tell
end tell