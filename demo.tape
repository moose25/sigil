# sigil demo — generates demo.gif
#
# Requires VHS (https://github.com/charmbracelet/vhs) and `sigil` on your PATH:
#   cargo install --path .
#   vhs demo.tape
#
# The result (demo.gif) is what the README's animated preview points to.

Output demo.gif

Set Shell bash
Set FontSize 20
Set Width 1200
Set Height 700
Set Padding 24
Set Theme "Catppuccin Mocha"

Type "sigil 'sigil' -f ansishadow -g vaporwave"
Enter
Sleep 2s

Type "sigil 'boxed' -b round -g ocean"
Enter
Sleep 2s

Type "sigil 'launch' -f small -g fire --animate sweep"
Enter
Sleep 4s

Type "sigil 'ready' --animate type --fps 60"
Enter
Sleep 3s
