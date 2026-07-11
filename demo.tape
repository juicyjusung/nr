# Demo for nr - Interactive npm script runner
# Shows project-wide fuzzy search and monorepo package navigation

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
Type "cd examples/demo-project"
Enter
Sleep 300ms
Type "export PATH=../../target/release:$PATH"
Enter
Sleep 200ms
Type "clear"
Enter
Show

Sleep 300ms

# Start from a workspace-only monorepo root
Type@30ms "nr"
Sleep 400ms
Enter
Sleep 1.5s

# Fuzzy search across workspace scripts
Type@100ms "tst"
Sleep 1.5s

Backspace@60ms 3
Sleep 600ms

Type@100ms "bld"
Sleep 1.5s

Backspace@60ms 3
Sleep 500ms

# Navigate and favorite a unified task
Down
Sleep 500ms
Down
Sleep 500ms
Space
Sleep 1s

Up
Sleep 400ms
Down
Sleep 400ms

# Retained package-first navigation
Right
Sleep 1.5s

Down
Sleep 600ms
Down
Sleep 600ms

Enter
Sleep 1.5s

Down
Sleep 500ms
Down
Sleep 500ms

Escape
Sleep 800ms

Left
Sleep 1s

Escape
Sleep 500ms
