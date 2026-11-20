# Reflection

## 1. Short-circuit `and` and `or`

`and` and `or` share one arm, `src/interpreter.rs:263-272`. The left operand is evaluated first. For `or`, `src/interpreter.rs:266-268` returns that value when it is truthy, and the right is not evaluated. For `and`, `src/interpreter.rs:269-271` returns the left value when it is falsey. Otherwise `src/interpreter.rs:272` evaluates the right operand and that value is the result.

`nil or "fallback"` evaluates the left to nil. Nil is not truthy, so control falls through and the expression yields `"fallback"`. `0 and 1` evaluates the left to 0. Zero is truthy, because only `nil` and `false` are false, so `and` does not return 0. It evaluates the right operand and yields `1`.

Returning `true` or `false` would still pass a test that only used the result as an `if` condition, because the truthiness would match. It would fail `tests/phase-3/valid/logical.kobo`, which prints the operand. `nil or "fallback"` must print `fallback`, and `false and 1` must print `false`.

That return was introduced in `26b62aa` as `return Ok(l);` at `src/interpreter.rs:181`. In `3235010` it is still `return Ok(l);`, now at `src/interpreter.rs:267`. The edit in between only added the const-scope argument to the call above it.

## 2. Slots, and one `Pop` too few

`lookup` (`src/resolver.rs:89-99`) walks the locals from the inside out, skips a binding that is still initialising, and records the winning declaration as a depth and a stack slot. `end_scope` (`src/compiler.rs:322-327`) emits one `Pop` for each local the resolver counted on that block. At run time `GET_LOCAL` is an index from the frame base (`src/vm.rs:105-108`). The VM does not search a map of names on every read, which is what the tree walk still does, so a local access stays a constant-time slot read however many scopes are open.

If `end_scope` emits one `Pop` too few, the dying local stays on the stack and every later slot sits one place too high. The tree walk throws away the whole scope map, so the bindings underneath are untouched and phase 3 still passes. Phase 4 does not. A loop that declares a fresh local on each iteration, `tests/phase-4/valid/jumps_while.kobo`, leaves that value behind, and the next iteration reads the wrong slot.

## 3. `const`

My matriculation number 24120111038 ends in 8, so the extension is `const`. The keyword is `src/token.rs:44` and `src/token.rs:113`. `src/parser.rs:128-136` parses `const name = expression ;`. A missing initialiser is the parse error `Expect initialiser after constant name.` The flag is `src/ast.rs:105`, printed at `src/printer.rs:26`. The tree walk marks the name at `src/interpreter.rs:128` and, when the assignment runs, reports `Cannot assign to constant 'name'.` at `src/interpreter.rs:203-207`. The right-hand side runs first (`src/interpreter.rs:198`). A later `var` or `fun` in that scope clears the mark.

The bytecode path was harder. A local is only a slot, so the compiler emits `MarkConst` or `DefineConst` at `src/compiler.rs:294-310` (opcodes `src/chunk.rs:28` and `src/chunk.rs:37`; `src/disassembler.rs:39-47` prints them). The VM refuses the store at `src/vm.rs:93-116`. A plain `SetLocal` would write through the constant. A block ends with `Pop`, which does not free the mark, so a `var` that reuses the slot must emit `MarkMutable` (`src/compiler.rs:299`, `src/vm.rs:125`) or the next assignment is rejected. The virtual machine did not get `const` for free. The behaviour landed in `3235010`; the tests are `08a3e4a`.
