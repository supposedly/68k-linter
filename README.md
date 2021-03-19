This isn't actual project code for 422. It's just a formatter for X68 files. It also doesn't work at the moment, but either I'll change that soon or anyone reading this can feel free to fork and fixxxx

## Goals
1. Snap instructions, their arguments, and their comments (i.e. non-line comments) to consistent tabstops
2. Enforce colons after labels that are alone on a line, which would let Ctrl+F jump to subroutines easily
3. Make it possible to enforce one single prefix for all comments
4. Warn on instructions that don't have a size code (didn't attempt to implement this in the end, but would be dead-easy to)
5. Compress consecutive invocations of `MOVE.B '*',(A5)+` to `MOVE.L '****',(A5)+` and `MOVE.W '**',(A5)+` where possible (this one is totally specific to our own program)

## Results
When the program is invoked as `cargo run filename.X68 > out.X68` (a release binary and a more-intuitive CLI would've been created if there'd been time), `out.X68` assembles fine and definitely looks correct at first glance, but feature #5 causes instant memory-access errors due to some oversight I haven't looked into yet. That feature isn't the only culprit, though, because commenting out its implementation reveals that the linter still introduces subtle bugs into the assembly created by the other transformation rules.

It may have been smarter to build an AST out of the code rather than work on the text representation directly. Either way, though, all of those issues are fixable given a bit of time and a bit more motivation.
