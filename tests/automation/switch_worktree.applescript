-- Activate pmux and simulate worktree switch (click second item in sidebar)
tell application "System Events"
	tell process "pmux"
		set frontmost to true
	end tell
end tell
delay 2

-- Try to find and click sidebar items (worktrees)
tell application "System Events"
	tell process "pmux"
		try
			-- Get all buttons/rows - worktrees might be in a list
			set allButtons to every button of window 1
			if (count of allButtons) > 1 then
				click button 2
			end if
		end try
	end tell
end tell
delay 2

-- Switch back to first
tell application "System Events"
	tell process "pmux"
		try
			click button 1
		end try
	end tell
end tell
delay 1
