Source tapes/common.tape
Set Height 600
Output out/check-convert.gif
Hide
Type "cd scratch/check-convert && export PS1='$ ' && clear"
Enter
Sleep 500ms
Show
Type "cat orders.sql"
Enter
Sleep 1500ms
Type "sqlquack check orders.sql"
Sleep 300ms
Enter
Wait@60s /\$\s*$/
Sleep 5s
Hide
Type "clear"
Enter
Sleep 300ms
Show
Type "sqlquack convert revenue.sql"
Sleep 300ms
Enter
Wait@60s /\$\s*$/
Sleep 4s
Type "sqlquack convert bucket.sql"
Sleep 300ms
Enter
Wait@60s /\$\s*$/
Sleep 1s
Screenshot out/check-convert.png
Sleep 5s
