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

on delayed_keystroke_ctrl_d()
	tell application "System Events"
		keystroke "d" using control down
		my delay_sec(typing_speed)
	end tell
end delayed_keystroke_ctrl_d

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

on activate_tab_by_id(id)
	my delayed_keystroke(id)
end activate_tab_by_id

on demo()
	my delay_sec(1)
	
	my delayed_keystroke("G")
	my delay_sec(1)
	
	my delayed_keystroke(return)
	my delay_sec(1)
	
	my delayed_keystroke(tab)
	my delay_sec(1)
	
	repeat 5 times
		my delayed_keystroke_ctrl_d()
		my delay_sec(0.5)
	end repeat
	
end demo

tell application "iTerm"
	activate
	tell current window
		tell current session of current tab
			my demo()
		end tell
	end tell
end tell