use anyhow::{Context, Result};
use cranelift_codegen::ir::{condcodes::IntCC, AbiParam, InstBuilder, UserFuncName, Value, types};
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;
use crate::frontend::{Expr, Function};

pub fn emit_object(function: &Function) -> Result<Vec<u8>> {
    let triple = Triple::host(); let mut flags = settings::builder(); flags.set("is_pic", "true")?;
    let isa = isa::lookup(triple.clone())?.finish(settings::Flags::new(flags))?;
    let mut module = ObjectModule::new(ObjectBuilder::new(isa, "ncc", default_libcall_names())?);
    let mut ctx = module.make_context(); ctx.func.signature.returns.push(AbiParam::new(types::I32)); ctx.func.name = UserFuncName::user(0, 0);
    let mut fb_ctx = FunctionBuilderContext::new();
    { let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx); let entry = b.create_block(); b.switch_to_block(entry); b.seal_block(entry); let value = lower_expr(&function.return_value, &mut b); b.ins().return_(&[value]); b.finalize(); }
    let id = module.declare_function(&function.name, Linkage::Export, &ctx.func.signature)?; module.define_function(id, &mut ctx)?; module.clear_context(&mut ctx);
    module.finish().emit().context("failed to emit object file")
}

fn lower_expr(expr: &Expr, b: &mut FunctionBuilder<'_>) -> Value {
    match expr {
        Expr::Integer(v) => b.ins().iconst(types::I32, *v),
        Expr::Neg(x) => { let v = lower_expr(x, b); b.ins().ineg(v) },
        Expr::Add(l, r) => bin(l, r, b, |b, l, r| b.ins().iadd(l, r)),
        Expr::Sub(l, r) => bin(l, r, b, |b, l, r| b.ins().isub(l, r)),
        Expr::Mul(l, r) => bin(l, r, b, |b, l, r| b.ins().imul(l, r)),
        Expr::Div(l, r) => bin(l, r, b, |b, l, r| b.ins().sdiv(l, r)),
        Expr::Rem(l, r) => bin(l, r, b, |b, l, r| b.ins().srem(l, r)),
        Expr::Eq(l, r) => cmp(IntCC::Equal, l, r, b), Expr::Ne(l, r) => cmp(IntCC::NotEqual, l, r, b),
        Expr::Lt(l, r) => cmp(IntCC::SignedLessThan, l, r, b), Expr::Le(l, r) => cmp(IntCC::SignedLessThanOrEqual, l, r, b),
        Expr::Gt(l, r) => cmp(IntCC::SignedGreaterThan, l, r, b), Expr::Ge(l, r) => cmp(IntCC::SignedGreaterThanOrEqual, l, r, b),
    }
}
fn bin<F: FnOnce(&mut FunctionBuilder<'_>, Value, Value) -> Value>(l: &Expr, r: &Expr, b: &mut FunctionBuilder<'_>, f: F) -> Value { let l = lower_expr(l, b); let r = lower_expr(r, b); f(b, l, r) }
fn cmp(cc: IntCC, l: &Expr, r: &Expr, b: &mut FunctionBuilder<'_>) -> Value { let l = lower_expr(l, b); let r = lower_expr(r, b); let v = b.ins().icmp(cc, l, r); b.ins().uextend(types::I32, v) }
