# Demo for nr - Interactive npm script runner
# Shows fuzzy search and monorepo support

Output assets/demo.gif

Set Shell "bash"
Set FontSize 15
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 16
Set TypingSpeed 60ms

# Setup
Hide
Type "cd /Users/user/projects/juicy-nr/examples/demo-project"
Enter
Sleep 300ms
Type "export PATH=/Users/user/projects/juicy-nr/target/release:$PATH"
Enter
Sleep 200ms
Type "clear"
Enter
Show

Sleep 300ms

# Intro - quick typing
Type@30ms "nr"
Sleep 400ms
Enter

# Wait for load
Sleep 1.5s

# Fuzzy search demo - visible typing
Type@100ms "tst"
Sleep 1.5s

Backspace@60ms 3
Sleep 600ms

Type@100ms "bld"
Sleep 1.5s

Backspace@60ms 3
Sleep 500ms

# Navigation
Down
Sleep 500ms
Down
Sleep 500ms

# Favorite
Space
Sleep 1s

Up
Sleep 400ms
Down
Sleep 400ms

# Packages tab
Right
Sleep 1.5s

Down
Sleep 600ms
Down
Sleep 600ms

# View package
Enter
Sleep 1.5s

Down
Sleep 500ms
Down
Sleep 500ms

# Back
Escape
Sleep 800ms

Left
Sleep 1s

# Exit
Sleep 1s
Escape
Sleep 500ms
