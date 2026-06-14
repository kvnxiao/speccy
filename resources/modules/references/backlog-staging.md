## Staging the inherited backlog

Stage `.speccy/BACKLOG.md` whenever it exists, so a candidate this loop touched
— one this skill appended or struck, or one a preceding brainstorm session
appended and left dirty — rides into this commit rather than persisting as an
uncommitted working-tree change. Guard the add on the file's existence:
`git add` on an unchanged path is a no-op, and the guard also catches a
first-append `.speccy/BACKLOG.md` that is still untracked, which `git diff`
would miss:

```bash
test -f .speccy/BACKLOG.md && git add .speccy/BACKLOG.md
```
