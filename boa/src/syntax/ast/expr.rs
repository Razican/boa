use crate::syntax::ast::{
    constant::Const,
    op::{BinOp, Operator, UnaryOp},
};
use gc_derive::{Finalize, Trace};
use std::{
    collections::btree_map::BTreeMap,
    fmt::{Display, Formatter, Result},
};

#[derive(Clone, Trace, Finalize, Debug, PartialEq)]
pub struct Expr {
    /// The expression definition
    pub def: ExprDef,
}

impl Expr {
    /// Create a new expression with a starting and ending position
    pub fn new(def: ExprDef) -> Self {
        Self { def }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.def)
    }
}

#[derive(Clone, Debug, Trace, Finalize, PartialEq)]
/// A Javascript Expression
pub enum ExprDef {
    /// Run a operation between 2 expressions
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    /// Run an operation on a value
    UnaryOp(UnaryOp, Box<Expr>),
    /// Make a constant value
    Const(Const),
    /// Const declaration
    ConstDecl(Vec<(String, Expr)>),
    /// Construct an object from the function and arg{
    Construct(Box<Expr>, Vec<Expr>),
    /// Run several expressions from top-to-bottom
    Block(Vec<Expr>),
    /// Load a reference to a value, or a function argument
    Local(String),
    /// Gets the constant field of a value
    GetConstField(Box<Expr>, String),
    /// Gets the field of a value
    GetField(Box<Expr>, Box<Expr>),
    /// Call a function with some values
    Call(Box<Expr>, Vec<Expr>),
    /// Repeatedly run an expression while the conditional expression resolves to true
    WhileLoop(Box<Expr>, Box<Expr>),
    /// Check if a conditional expression is true and run an expression if it is and another expression if it isn't
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    /// Run blocks whose cases match the expression
    Switch(Box<Expr>, Vec<(Expr, Vec<Expr>)>, Option<Box<Expr>>),
    /// Create an object out of the binary tree given
    ObjectDecl(Box<BTreeMap<String, Expr>>),
    /// Create an array with items inside
    ArrayDecl(Vec<Expr>),
    /// Create a function with the given name, arguments, and expression
    FunctionDecl(Option<String>, Vec<Expr>, Box<Expr>),
    /// Create an arrow function with the given arguments and expression
    ArrowFunctionDecl(Vec<Expr>, Box<Expr>),
    /// Return the expression from a function
    Return(Option<Box<Expr>>),
    /// Throw a value
    Throw(Box<Expr>),
    /// Assign an expression to a value
    Assign(Box<Expr>, Box<Expr>),
    /// A variable declaration
    VarDecl(Vec<(String, Option<Expr>)>),
    /// Let declaraton
    LetDecl(Vec<(String, Option<Expr>)>),
    /// Return a string representing the type of the given expression
    TypeOf(Box<Expr>),
    /// Try...catch...finally block.
    TryCatch(
        Box<Expr>,
        Option<(Option<String>, Box<Expr>)>,
        Option<Box<Expr>>,
    ),
}

impl Operator for ExprDef {
    fn get_assoc(&self) -> bool {
        match *self {
            ExprDef::Construct(_, _)
            | ExprDef::UnaryOp(_, _)
            | ExprDef::TypeOf(_)
            | ExprDef::If(_, _, _)
            | ExprDef::Assign(_, _) => false,
            _ => true,
        }
    }
    fn get_precedence(&self) -> u64 {
        match self {
            ExprDef::GetField(_, _) | ExprDef::GetConstField(_, _) => 1,
            ExprDef::Call(_, _) | ExprDef::Construct(_, _) => 2,
            ExprDef::UnaryOp(UnaryOp::IncrementPost, _)
            | ExprDef::UnaryOp(UnaryOp::IncrementPre, _)
            | ExprDef::UnaryOp(UnaryOp::DecrementPost, _)
            | ExprDef::UnaryOp(UnaryOp::DecrementPre, _) => 3,
            ExprDef::UnaryOp(UnaryOp::Not, _)
            | ExprDef::UnaryOp(UnaryOp::Tilde, _)
            | ExprDef::UnaryOp(UnaryOp::Minus, _)
            | ExprDef::TypeOf(_) => 4,
            ExprDef::BinOp(op, _, _) => op.get_precedence(),
            ExprDef::If(_, _, _) => 15,
            // 16 should be yield
            ExprDef::Assign(_, _) => 17,
            _ => 19,
        }
    }
}

impl Display for ExprDef {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.display(f, 0)
    }
}

impl ExprDef {
    fn display(&self, f: &mut Formatter, indentation: usize) -> Result {
        let indent = "    ".repeat(indentation);
        match *self {
            Self::Block(_) => {}
            _ => write!(f, "{}", indent)?,
        }

        match *self {
            Self::Const(ref c) => write!(f, "{}", c),
            Self::Block(ref block) => {
                writeln!(f, "{{")?;
                for expr in block.iter() {
                    expr.def.display(f, indentation + 1)?;

                    match expr.def {
                        Self::Block(_)
                        | Self::If(_, _, _)
                        | Self::Switch(_, _, _)
                        | Self::FunctionDecl(_, _, _)
                        | Self::WhileLoop(_, _)
                        | Self::TryCatch(_, _, _) => {}
                        _ => write!(f, ";")?,
                    }
                    writeln!(f)?;
                }
                write!(f, "{}}}", indent)
            }
            Self::Local(ref s) => write!(f, "{}", s),
            Self::GetConstField(ref ex, ref field) => write!(f, "{}.{}", ex, field),
            Self::GetField(ref ex, ref field) => write!(f, "{}[{}]", ex, field),
            Self::Call(ref ex, ref args) => {
                write!(f, "{}(", ex)?;
                let arg_strs: Vec<String> = args.iter().map(ToString::to_string).collect();
                write!(f, "{})", arg_strs.join(", "))
            }
            Self::Construct(ref func, ref args) => {
                write!(f, "new {}", func)?;
                f.write_str("(")?;
                let mut first = true;
                for e in args.iter() {
                    if !first {
                        f.write_str(", ")?;
                        first = false;
                    }

                    Display::fmt(e, f)?;
                }
                f.write_str(")")
            }
            Self::WhileLoop(ref cond, ref expr) => {
                write!(f, "while ({}) ", cond)?;
                expr.def.display(f, indentation)
            }
            Self::If(ref cond, ref expr, None) => {
                write!(f, "if ({}) ", cond)?;
                expr.def.display(f, indentation)
            }
            Self::If(ref cond, ref expr, Some(ref else_e)) => {
                write!(f, "if ({}) ", cond)?;
                expr.def.display(f, indentation)?;
                f.write_str(" else ")?;
                else_e.def.display(f, indentation)
            }
            Self::Switch(ref val, ref vals, None) => {
                writeln!(f, "switch ({}) {{", val)?;
                for e in vals.iter() {
                    writeln!(f, "{}case {}:", indent, e.0)?;
                    join_expr(f, &e.1)?;
                }
                writeln!(f, "{}}}", indent)
            }
            Self::Switch(ref val, ref vals, Some(ref def)) => {
                writeln!(f, "switch ({}) {{", val)?;
                for e in vals.iter() {
                    writeln!(f, "{}case {}:", indent, e.0)?;
                    join_expr(f, &e.1)?;
                }
                writeln!(f, "{}default:", indent)?;
                def.def.display(f, indentation + 1)?;
                write!(f, "{}}}", indent)
            }
            Self::ObjectDecl(ref map) => {
                f.write_str("{\n")?;
                for (key, value) in map.iter() {
                    write!(f, "{}    {}: {},", indent, key, value)?;
                }
                f.write_str("}")
            }
            Self::ArrayDecl(ref arr) => {
                f.write_str("[")?;
                join_expr(f, arr)?;
                f.write_str("]")
            }
            Self::FunctionDecl(ref name, ref args, ref expr) => {
                write!(f, "function ")?;
                if let Some(func_name) = name {
                    write!(f, "{}", func_name)?;
                }
                write!(f, "{{")?;
                join_expr(f, args)?;
                f.write_str("} ")?;
                expr.def.display(f, indentation + 1)
            }
            Self::ArrowFunctionDecl(ref args, ref expr) => {
                write!(f, "(")?;
                join_expr(f, args)?;
                f.write_str(") => ")?;
                expr.def.display(f, indentation)
            }
            Self::BinOp(ref op, ref a, ref b) => write!(f, "{} {} {}", a, op, b),
            Self::UnaryOp(ref op, ref a) => write!(f, "{}{}", op, a),
            Self::Return(Some(ref ex)) => write!(f, "return {}", ex),
            Self::Return(None) => write!(f, "return"),
            Self::Throw(ref ex) => write!(f, "throw {}", ex),
            Self::Assign(ref ref_e, ref val) => write!(f, "{} = {}", ref_e, val),
            Self::VarDecl(ref vars) | Self::LetDecl(ref vars) => {
                if let Self::VarDecl(_) = *self {
                    f.write_str("var ")?;
                } else {
                    f.write_str("let ")?;
                }
                for (key, val) in vars.iter() {
                    match val {
                        Some(x) => write!(f, "{} = {}", key, x)?,
                        None => write!(f, "{}", key)?,
                    }
                }
                Ok(())
            }
            Self::ConstDecl(ref vars) => {
                f.write_str("const ")?;
                for (key, val) in vars.iter() {
                    write!(f, "{} = {}", key, val)?
                }
                Ok(())
            }
            Self::TypeOf(ref e) => write!(f, "typeof {}", e),
            Self::TryCatch(ref try_block, ref catch_block, ref finally_block) => {
                f.write_str("try ")?;
                try_block.def.display(f, indentation)?;
                if let Some((catch_binding, catch_expr)) = catch_block {
                    f.write_str(" catch ")?;
                    if let Some(exc) = catch_binding {
                        write!(f, "({}) ", exc)?;
                    }
                    catch_expr.def.display(f, indentation)?;
                }
                if let Some(finally) = finally_block {
                    f.write_str(" finally ")?;
                    finally.def.display(f, indentation)?;
                }

                Ok(())
            }
        }
    }
}

/// `join_expr` - Utility to join multiple Expressions into a single string
fn join_expr(f: &mut Formatter, expr: &[Expr]) -> Result {
    let mut first = true;
    for e in expr.iter() {
        if !first {
            f.write_str(", ")?;
        }
        first = false;
        Display::fmt(e, f)?;
    }
    Ok(())
}
