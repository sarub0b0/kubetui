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

on demo()
	my delay_sec(1)
	
	-- pod:app
	
	delayed_keystroke("pod")
	delayed_keycode(39)
	delayed_keystroke("app")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(2)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("g")
	my delay_sec(2)
	
	delayed_keystroke_backtab()
	my delay_sec(1)
	
	delayed_keystroke_ctrl_w()
	my delay_sec(1)
	
	-- label:app.kubernetes.io/instance=my-prometheus
	
	delayed_keystroke("label")
	delayed_keycode(39)
	delayed_keystroke("app.kubernetes.io/instance=my-prometheus")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(2)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("g")
	my delay_sec(2)
	
	delayed_keystroke_backtab()
	my delay_sec(1)
	
	delayed_keystroke_ctrl_w()
	my delay_sec(1)
	
	-- deploy/app
	delayed_keystroke("deploy/app")
	my delay_sec(1)
	
	delayed_keystroke(return)
	my delay_sec(2)
	
	delayed_keystroke(tab)
	my delay_sec(1)
	
	delayed_keystroke("g")
	my delay_sec(2)
	
	delayed_keystroke_backtab()
	my delay_sec(1)
	
	delayed_keystroke_ctrl_w()
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