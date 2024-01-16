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
	my pod_tab_open_yaml()
end pod_tab

on pod_tab_open_yaml()
	delayed_keystroke("y")
	my delay_sec(2)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end pod_tab_open_yaml

on activate_tab_by_id(id)
	my delayed_keystroke(id)
end activate_tab_by_id

on config_tab_open_yaml()
	delayed_keystroke("y")
	my delay_sec(2)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end config_tab_open_yaml

on config_tab()
	config_tab_open_yaml()
end config_tab

on network_tab_open_yaml()
	delayed_keystroke("y")
	my delay_sec(2)
	
	delayed_keystroke_escape()
	my delay_sec(1)
end network_tab_open_yaml

on network_tab()
	my network_tab_open_yaml()
end network_tab

on demo()
	my delay_sec(1)
	
	my pod_tab()
	
	my activate_tab_by_id("2")
	my delay_sec(1)
	
	my config_tab()
	
	my activate_tab_by_id("3")
	my delay_sec(1)
	
	my network_tab()
	
	my activate_tab_by_id("1")
	my delay_sec(1)
end demo

tell application "iTerm"
	activate
	tell current window
		tell current session of current tab
			my demo()
		end tell
	end tell
end tell