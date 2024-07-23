global playback_speed, typing_speed

set playback_speed to 1.0

set typing_speed to 0.05

on sec(s)
	return s * playback_speed
end sec

on delay_sec(s)
	delay sec(s)
end delay_sec

on toggle_split_direction()
	tell application "System Events"
		keystroke "s" using shift down
		my delay_sec(typing_speed)
	end tell
end toggle_split_direction

on demo()
	my delay_sec(1)
	
	my toggle_split_direction()
	
	my delay_sec(1)
	
	my toggle_split_direction()
	
	my delay_sec(1)
	
	my toggle_split_direction()
	
	my delay_sec(1)
	
	my toggle_split_direction()
	
	my delay_sec(1)
	
	my toggle_split_direction()
	
	my delay_sec(1)
	
	my toggle_split_direction()
	
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
