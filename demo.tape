# Crusty Demo — VHS tape file
# Run with: vhs demo.tape
# Requires: vhs (github.com/charmbracelet/vhs)

Output demo.gif
Set Theme "Dracula"
Set FontSize 14
Set Width 1400
Set Height 800
Set Padding 10
Set TypingSpeed 60ms
Set PlaybackSpeed 0.5

# Build first so startup is instant
Hide
Type "cargo build -p crusty-tui --release 2>/dev/null && clear"
Enter
Sleep 10s
Show

# Start the app
Type "cargo run -p crusty-tui --release"
Enter
Sleep 2s

# --- 1. Send a GET request ---
# App starts focused on UrlBar
Type "https://jsonplaceholder.typicode.com/posts/1"
Sleep 500ms

# Send via Enter
Enter
Sleep 3s

# Show the 200 OK JSON response
Sleep 2s

# --- 2. Browse request tabs ---
# Esc to ResponseBody (number keys only work here)
Escape
Sleep 500ms

# Cycle through request tabs: Headers, Body, Auth, back to Params
Type "2"
Sleep 1.2s
Type "3"
Sleep 1.2s
Type "4"
Sleep 1.2s
Type "1"
Sleep 1s

# --- 3. Toggle sidebar ---
Ctrl+b
Sleep 1.5s
Ctrl+b
Sleep 1s

# --- 4. Open method selector and switch to POST ---
# From ResponseBody, press m to open method selector
Type "m"
Sleep 500ms
Down
Enter
Sleep 500ms

# --- 5. Edit URL for POST endpoint ---
# Enter from ResponseBody goes to UrlBar
Enter
Sleep 300ms

# Go to end of URL, remove "/1" to POST to /posts
Backspace
Backspace
Sleep 300ms

# Send the POST
Enter
Sleep 3s
Sleep 1.5s

# --- 6. Generate code snippet ---
Ctrl+g
Sleep 2.5s
Escape
Sleep 1s

# --- 7. Show help overlay ---
# Esc to ResponseBody first
Escape
Sleep 300ms
Type "?"
Sleep 3s
Escape
Sleep 1s

# --- 8. Cycle environment ---
Ctrl+e
Sleep 1.5s

# Exit
Ctrl+c
Sleep 500ms
