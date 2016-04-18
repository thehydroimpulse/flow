#![crate_type="dylib"]
#![feature(plugin_registrar, quote, rustc_private)]

extern crate syntax;
extern crate rustc;
extern crate rustc_plugin;

use std::mem;
use std::iter::Iterator;
use syntax::codemap::Span;
use syntax::parse::{self, parser};
use syntax::parse::token::{self, Token};
use syntax::parse::token::keywords;
use syntax::ast::{self, TokenTree};
use syntax::ptr::P;
use syntax::ext::base::{ExtCtxt, MacResult, DummyResult, MacEager};
use syntax::ext::build::AstBuilder;  // trait for expr_usize
use rustc_plugin::Registry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// Simple asynchronous statement that we want a value
    /// extracted from.
    ///
    /// ```notrust
    /// flow! {
    ///     let a <- foobar
    /// }
    /// ```
    Async(ast::Ident, P<ast::Expr>),
    /// Normal expression that belongs to a parent block.
    /// XXX: We want to somehow detect if the expression has dependencies
    /// on previously extracted values. If not, then we can make this statement parallel.
    Expr(P<ast::Expr>),
    Ident(ast::Ident)
}

pub enum State {
    Begin,
    ParsingBlock(Block)
}

pub struct Flow<'a: 'x, 'x> {
    inner: parser::Parser<'a>,
    cx: &'x mut ExtCtxt<'a>,
    state: State,
    blocks: Vec<Block>
}

impl<'a, 'x> Flow<'a, 'x> {
    pub fn new(cx: &'x mut ExtCtxt<'a>, args: &[TokenTree]) -> Flow<'a, 'x> {
        Flow::<'a, 'x> {
            inner: cx.new_parser_from_tts::<'a>(args),
            cx: cx,
            state: State::Begin,
            blocks: Vec::new()
        }
    }

    /// flow! {
    ///     a <- foo
    ///     b <- big(a)
    /// }
    ///
    /// Block::Root(vec![
    ///     Block::Async(a, foo),
    ///     Block::Async(b, big(a))
    /// ])
    ///

    /// Parse the next block.
    pub fn parse_block(&mut self) -> Option<Block> {
        // Do we have an expression waiting?
        if self.inner.eat_keyword(keywords::Let) {
            self.parse_async()
        } else if self.inner.eat(&token::Eof) {
            None
        } else {
            self.parse_expr()
        }
    }

    pub fn parse(&mut self) {
        while let Some(block) = self.parse_block() {
            self.blocks.push(block);
        }
    }

    fn get_ident_from_pat(&mut self, pat: P<ast::Pat>) -> ast::Ident {
        match pat.node {
            ast::PatKind::Ident(mode, ref span, ref p) => {
                span.node
            },
            _ => panic!("Error")
        }
    }

    fn code(&mut self) -> Box<MacResult + 'static> {
        let mut blocks = mem::replace(&mut self.blocks, Vec::new());
        blocks.reverse();

        let expr = blocks.iter().fold(quote_expr!(self.cx, {}), |acc, ref block| {
            match block {
                &&Block::Async(ref ident, ref expr) => {
                    quote_expr!(self.cx, {
                        ($expr).and_then(move |a| {
                            Async::Ok({$acc})
                        })
                    })
                },
                &&Block::Expr(ref expr) => {
                    println!("{:?}", expr);
                    quote_expr!(self.cx, {
                        $acc;
                        $expr
                    })
                },
                &&Block::Ident(ref ident) => {
                    quote_expr!(self.cx, {
                        $acc;
                        $ident
                    })
                }
            }
        });

        MacEager::expr(expr)
    }

    fn parse_async(&mut self) -> Option<Block> {
        let pat = self.inner.parse_pat().unwrap();

        let mut ty = None;

        if self.inner.eat(&token::Colon) {
            ty = Some(self.inner.parse_ty_sum().unwrap());
        }

        if !self.inner.eat(&token::LArrow) {
            // cx.span_err(sp, &format!("expected `<` token"));
            // return DummyResult::any(sp);
            return None;
        }

        let expr = self.inner.parse_expr().unwrap();
        match &expr.node {
            &ast::ExprKind::Call(ref expr, ref args) => {
                match &args[0].node {
                    &ast::ExprKind::Path(ref q, ref path) => {
                        println!("Expr: {:?}", path.segments[0].identifier);
                    },
                    _ => {}
                }
            },
            _ => {}
        }
        let ident = self.get_ident_from_pat(pat);

        Some(Block::Async(ident, expr))
    }

    fn parse_expr(&mut self) -> Option<Block> {

        match self.inner.parse_ident() {
            Ok(ident) => {
                println!("{:?}", ident);
                return Some(Block::Ident(ident));
            },
            Err(err) => {}
        }

        let expr = match self.inner.parse_expr() {
            Ok(expr) => Some(Block::Expr(expr)),
            Err(err) => return None
        };

        self.inner.eat(&token::Semi);

        expr
    }
}

fn expand_rn(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    if args.len() == 0 {
        // XXX: Return an empty future.
        return DummyResult::any(sp);
    }

    let mut flow = Flow::new(cx, args);
    flow.parse();
    flow.code()
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("flow", expand_rn);
}
