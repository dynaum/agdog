# vhs tape for the agdog demo GIF.
# Regenerate with: vhs docs/demo.tape   (from the repo root)

Output docs/demo.gif

Set Shell "bash"
Set FontSize 13
Set Width 1240
Set Height 860
Set Padding 12
Set Theme "Catppuccin Mocha"

Type "AGDOG_DEMO=1 ./target/release/agdog --gpu-hourly 2.50"
Enter
Sleep 6s
Type "s"
Sleep 2s
Type "s"
Sleep 2s
Type "q"
Sleep 1s
