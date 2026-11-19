use std::collections::HashSet;

use crate::ast::{Expr, Program, Resolved, Stmt};
use crate::environment::Environment;
use crate::token::{Token, TokenType};
use crate::value::Value;

pub struct RuntimeError {
    pub line: usize,
    pub message: String,
}

/// Why a statement stopped early. An error unwinds to `main`; a `return` unwinds only as
/// far as the call that started the function, which is why both travel the same way. Rust
/// gives `?` for free on a `Result`, and a `return` is exactly a non-local exit.
enum Flow {
    Error(RuntimeError),
    Return(Value),
}

type Exec<T> = Result<T, Flow>;

/// Kobo's call stack is the Rust call stack: `call` calls `execute` calls `evaluate`
/// calls `call`. Running off the end of it aborts the process with no diagnostic, so the
/// depth is capped instead. Measured on this toolchain, the real limit is near 1,500
/// frames; 500 leaves room for a deeply nested expression inside every one of them.
pub const MAX_FRAMES: usize = 500;

/// Which names in each live scope were bound with `const`. Kept beside `Environment`
/// because that file is given and stores values only. A `var` or `fun` for the same
/// name removes the mark, which is Kobo's redeclaration rule applied to const.
struct Consts {
    scopes: Vec<HashSet<String>>,
    frames: Vec<usize>,
}

impl Consts {
    fn new() -> Self {
        Consts {
            scopes: vec![HashSet::new()],
            frames: Vec::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn push_frame(&mut self) {
        self.frames.push(self.scopes.len());
        self.scopes.push(HashSet::new());
    }

    fn pop_frame(&mut self) {
        if let Some(base) = self.frames.pop() {
            self.scopes.truncate(base);
        }
    }

    fn bind(&mut self, name: &str, constant: bool) {
        let scope = self.scopes.last_mut().expect("a scope is always open");
        if constant {
            scope.insert(name.to_string());
        } else {
            scope.remove(name);
        }
    }

    fn is_const_at(&self, depth: usize, name: &str) -> bool {
        match self.scopes.len().checked_sub(depth + 1) {
            Some(i) => self.scopes[i].contains(name),
            None => false,
        }
    }

    fn is_const_global(&self, name: &str) -> bool {
        self.scopes[0].contains(name)
    }
}

pub fn interpret(program: &Program) -> Result<(), RuntimeError> {
    let mut env = Environment::new();
    let mut consts = Consts::new();
    for stmt in &program.stmts {
        match execute(stmt, &mut env, &mut consts, program) {
            Ok(()) => {}
            // `return` outside any function ends the program (spec section 3.1).
            Err(Flow::Return(_)) => return Ok(()),
            Err(Flow::Error(e)) => return Err(e),
        }
    }
    Ok(())
}

fn execute(
    stmt: &Stmt,
    env: &mut Environment,
    consts: &mut Consts,
    program: &Program,
) -> Exec<()> {
    match stmt {
        Stmt::Expression(expr) => {
            evaluate(expr, env, consts, program)?;
            Ok(())
        }

        Stmt::Print(expr) => {
            let value = evaluate(expr, env, consts, program)?;
            println!("{}", value);
            Ok(())
        }

        Stmt::Var {
            name,
            initialiser,
            constant,
            ..
        } => {
            let value = match initialiser {
                Some(expr) => evaluate(expr, env, consts, program)?,
                None => Value::Nil,
            };
            env.declare(&name.lexeme, value);
            consts.bind(&name.lexeme, *constant);
            Ok(())
        }

        Stmt::Block { body, .. } => {
            env.push_scope();
            consts.push_scope();
            for stmt in body {
                execute(stmt, env, consts, program)?;
            }
            env.pop_scope();
            consts.pop_scope();
            Ok(())
        }

        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            if evaluate(condition, env, consts, program)?.is_truthy() {
                execute(then_branch, env, consts, program)?;
            } else if let Some(alt) = else_branch {
                execute(alt, env, consts, program)?;
            }
            Ok(())
        }

        Stmt::While { condition, body } => {
            while evaluate(condition, env, consts, program)?.is_truthy() {
                execute(body, env, consts, program)?;
            }
            Ok(())
        }

        Stmt::Function { index, name, .. } => {
            env.declare(
                name,
                Value::Function {
                    index: *index,
                    name: name.clone(),
                },
            );
            consts.bind(name, false);
            Ok(())
        }

        Stmt::Return(value) => {
            let value = match value {
                Some(expr) => evaluate(expr, env, consts, program)?,
                None => Value::Nil,
            };
            Err(Flow::Return(value))
        }
    }
}

fn evaluate(
    expr: &Expr,
    env: &mut Environment,
    consts: &mut Consts,
    program: &Program,
) -> Exec<Value> {
    match expr {
        Expr::Literal { value, .. } => Ok(value.clone()),
        Expr::Grouping(inner) => evaluate(inner, env, consts, program),

        Expr::Variable { name, at } => read(env, name, *at),

        Expr::Assign { name, value, at } => {
            let value = evaluate(value, env, consts, program)?;
            let constant = match at {
                Resolved::Global => consts.is_const_global(&name.lexeme),
                Resolved::Local { depth, .. } => consts.is_const_at(*depth, &name.lexeme),
            };
            if constant {
                return Err(err(
                    name.line,
                    &format!("Cannot assign to constant '{}'.", name.lexeme),
                ));
            }
            let wrote = match at {
                Resolved::Global => env.assign_global(&name.lexeme, value.clone()),
                Resolved::Local { depth, .. } => {
                    env.assign_at(*depth, &name.lexeme, value.clone())
                }
            };
            if wrote {
                Ok(value)
            } else {
                Err(undefined(name))
            }
        }

        Expr::Unary { op, right } => {
            let value = evaluate(right, env, consts, program)?;
            match op.kind {
                TokenType::Bang => Ok(Value::Bool(!value.is_truthy())),
                TokenType::Minus => match value {
                    Value::Number(n) => Ok(Value::Number(-n)),
                    _ => Err(err(op.line, "Operand must be a number.")),
                },
                _ => Err(err(op.line, "Operand must be a number.")),
            }
        }

        Expr::Binary { left, op, right } => {
            let l = evaluate(left, env, consts, program)?;
            let r = evaluate(right, env, consts, program)?;
            match op.kind {
                TokenType::Plus => match (l, r) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                    (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
                    _ => Err(err(op.line, "Operands must be two numbers or two strings.")),
                },
                TokenType::Minus => arith(l, r, op.line, |a, b| a - b),
                TokenType::Star => arith(l, r, op.line, |a, b| a * b),
                TokenType::Slash => {
                    let (a, b) = numbers(l, r, op.line)?;
                    if b == 0.0 {
                        return Err(err(op.line, "Division by zero."));
                    }
                    Ok(Value::Number(a / b))
                }
                TokenType::Greater => compare(l, r, op.line, |a, b| a > b),
                TokenType::GreaterEqual => compare(l, r, op.line, |a, b| a >= b),
                TokenType::Less => compare(l, r, op.line, |a, b| a < b),
                TokenType::LessEqual => compare(l, r, op.line, |a, b| a <= b),
                TokenType::EqualEqual => Ok(Value::Bool(l == r)),
                TokenType::BangEqual => Ok(Value::Bool(l != r)),
                _ => Err(err(op.line, "Operands must be numbers.")),
            }
        }

        // `and` / `or` yield one of their operands, not a boolean (spec 4.1).
        Expr::Logical { left, op, right } => {
            let l = evaluate(left, env, consts, program)?;
            if op.kind == TokenType::Or {
                if l.is_truthy() {
                    return Ok(l);
                }
            } else if !l.is_truthy() {
                return Ok(l);
            }
            evaluate(right, env, consts, program)
        }

        Expr::Call {
            callee, paren, args, ..
        } => {
            let callee = evaluate(callee, env, consts, program)?;
            let mut values = Vec::new();
            for arg in args {
                values.push(evaluate(arg, env, consts, program)?);
            }
            match callee {
                Value::Function { index, .. } => call(index, values, paren.line, env, consts, program),
                _ => Err(err(paren.line, "This value is not a function.")),
            }
        }
    }
}

fn read(env: &Environment, name: &Token, at: Resolved) -> Exec<Value> {
    let value = match at {
        Resolved::Global => env.get_global(&name.lexeme),
        Resolved::Local { depth, .. } => env.get_at(depth, &name.lexeme),
    };
    match value {
        Some(value) => Ok(value.clone()),
        None => Err(undefined(name)),
    }
}

/// Run a function body in a fresh frame. Everything about Kobo's scope rules is in these
/// twenty lines: the parameters are declared in the frame's own scope, the body can reach
/// past it only to the globals, and the frame is popped on every path out: normal
/// completion, `return`, or a runtime error.
fn call(
    index: usize,
    args: Vec<Value>,
    line: usize,
    env: &mut Environment,
    consts: &mut Consts,
    program: &Program,
) -> Exec<Value> {
    let arity = program.functions[index].params.len();
    if args.len() != arity {
        return Err(err(
            line,
            &format!("Expected {} arguments but got {}.", arity, args.len()),
        ));
    }
    if env.frame_depth() >= MAX_FRAMES {
        return Err(err(line, "Stack overflow."));
    }

    let params = program.functions[index].params.clone();
    env.push_frame();
    consts.push_frame();
    for (name, value) in params.iter().zip(args) {
        env.declare(name, value);
        consts.bind(name, false);
    }

    for stmt in &program.functions[index].body {
        match execute(stmt, env, consts, program) {
            Ok(()) => {}
            Err(Flow::Return(value)) => {
                env.pop_frame();
                consts.pop_frame();
                return Ok(value);
            }
            Err(Flow::Error(error)) => {
                env.pop_frame();
                consts.pop_frame();
                return Err(Flow::Error(error));
            }
        }
    }

    env.pop_frame();
    consts.pop_frame();
    Ok(Value::Nil)
}

fn numbers(l: Value, r: Value, line: usize) -> Exec<(f64, f64)> {
    match (l, r) {
        (Value::Number(a), Value::Number(b)) => Ok((a, b)),
        _ => Err(err(line, "Operands must be numbers.")),
    }
}

fn arith(l: Value, r: Value, line: usize, f: fn(f64, f64) -> f64) -> Exec<Value> {
    let (a, b) = numbers(l, r, line)?;
    Ok(Value::Number(f(a, b)))
}

fn compare(l: Value, r: Value, line: usize, f: fn(f64, f64) -> bool) -> Exec<Value> {
    let (a, b) = numbers(l, r, line)?;
    Ok(Value::Bool(f(a, b)))
}

fn undefined(name: &Token) -> Flow {
    err(name.line, &format!("Variable '{}' is not defined.", name.lexeme))
}

fn err(line: usize, message: &str) -> Flow {
    Flow::Error(RuntimeError {
        line,
        message: message.to_string(),
    })
}
