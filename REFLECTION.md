1. Cite the line where you decide that a . begins a fractional part rather than being a stray
character. What does your scanner do with the input 5., and why is that what section 1.4
requires?
src/scanner.rs:157
With the input 5., the scanners goes through the input character by character, sees the 5 and treats it as a number.
Then it peeks at the dot, to check if it part of the number token, it does a lookahead to check for a number following the dot,
so its in-line with the defined pattern for numbers in kobo. It doesn't see a number after the dot, so it ends the token, and adds
5 as a Number token. The next iteration of the scanner sees the dot, which isn't a valid start to any token, and reports the
'Character is not part of any token.' error.

2. Cite every line where you change the line counter. Explain which line your EOF token ends
up carrying for a file that ends in two blank lines, and why section 6.1 asks for that rather than
the file's last line.
src/scanner.rs:35, src/scanner.rs:53, src/scanner.rs:147 

EOF token carries the line count of the file's last token, even with two blank lines at the end.
This is because EOF should mark the end of the token stream, not the end of the file itself.


3. Name one test in tests/phase-1/ that you failed at some point. Cite the line you changed
to fix it, and say plainly what you had misunderstood. “I made a typo” is not an answer; “I was
incrementing line before adding the token, so the token was reported on the next line” is.
Anchor it in your own history. Give the two commit hashes — the commit where that
line was still wrong, and the commit where you fixed it — and quote the line as it stood in
each. git log -p --follow src/scanner.rs will find them, and git log --oneline
gives you the hashes. Both commits must be your own and both must be at or before the
deadline. This is checked against your repository, so a hash that is not in your history makes
the answer incomplete.

I was failing invalid/trailing_dot.kobo. To fix it I reworked the whole number function, notably
src/scanner.rs:155-177. I naively advanced after seeing a dot, without doing a lookahead check
to see if it was followed by a number first.

Introduced in: c4690ac9d216ff8feeba89eb105674d261a74079
```rs
'.' => {
                    if has_dot {
                        self.error(self.line, "Invalid number"); ()
                    } else { has_dot = true; }
                    self.advance();
                },
```

Fixed in: 64f059d4950134b98a57eff7fab4342657aa9747
```rs
                .' => {
                    // only advance if next character is valid
                    match self.peek_next() {
                        '0'..='9' => self.advance(),
                        _ => break,
                    };
                    // once we've seen one dot, only treat subsequent numbers as valid characters
                    // break so match in scan_token handles anything after that
                    while !self.at_end() {
                        match self.peek() {
                            '0'..='9' => {
                                self.advance();
                            },
                            _ => break 'outer,
                        }
                    }
                }
```
