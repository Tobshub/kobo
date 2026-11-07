1. Cite the lines in your if_statement that resolve the dangling else. Explain why the
recursion decides it rather than the grammar, and what your parser prints for
if (false) if (true) print "a"; else print "b";, and what it would print if the else
bound to the outer if instead.
src/parser.rs:192, src/parser.rs:195

The then branch is a full statement() call, and only after that returns does this if look
for an else. A nested if is that statement, so its own if_statement is still running and
takes the else before the outer one gets to check. The grammar just says the else is
optional. It never says which if it belongs to. The call stack does.

For `if (false) if (true) print "a"; else print "b";` it prints:

(if false (if true (print "a") (print "b")))

If the else bound to the outer if, the inner one would have no else branch, and it would print:

(if false (if true (print "a")) (print "b"))

2. Cite the line in assignment where you check what the left side turned out to be.
Explain why that check cannot happen before the left side is parsed, and name the message
from section 5.1 that 1 = 2; produces.
src/parser.rs:266

The left side is already parsed, by or(), before that match. A name, a number and a call
all have to be read as an expression before you can see whether an = follows, and one
token of lookahead cannot tell those apart. The match then keeps the node only when it is
a bare variable. 1 = 2; takes the other branch and reports Cannot assign to this expression.

3. Cite synchronise. Say exactly which tokens you stop on and why those. Then name one
test in tests/phase-2/invalid/ whose output changes if you delete the call to it, and say
what its output becomes.
src/parser.rs:463, src/parser.rs:467

It steps over one token first, then stops when the token it just passed is `;`, or when
the token it is now sitting on is fun, var, if, while, print or return. A semicolon means
the broken statement is finished. Those six keywords are the ones that start a declaration
or a statement, so that is a real place to start parsing again.

The call is src/parser.rs:18. Deleting it changes
tests/phase-2/invalid/expect_variable_name.kobo. With the call, that test prints:

[line 1] Error at '123': Expect variable name.

Without it the parser is still on 123, so it parses 123 = 1 as an assignment and the
output becomes:

[line 1] Error at '123': Expect variable name.
[line 1] Error at '=': Cannot assign to this expression.

I was only returning from synchronise after a semicolon, so it never resumed on a statement
keyword and would keep discarding the next statement while it looked for one.

Still wrong in: f9dbff81c3b39607a33bed731193038b96a00b40
```rs
            if self.previous().kind == TokenType::Semicolon {
                return;
            }
```

Fixed in: b6263f1161330f47e5989f4c8b7d4df0ca8dbbd0
```rs
            match self.peek().kind {
                TokenType::Fun
                | TokenType::Var
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => return,
                _ => {}
            }
```
