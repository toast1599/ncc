use anyhow::{bail, Context, Result};
use cranelift_codegen::ir::{
    condcodes::IntCC, types, AbiParam, InstBuilder, UserFuncName, Value,
};
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{default_libcall_names, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;

use crate::frontend::{Expr, Function};

pub fn emit_object(function: &Function) -> Result<Vec<u8>> {
    let triple = Triple::host();
    let mut flags = settings::builder();
    flags.set("is_pic", "true")?;
    let isa = isa::lookup(triple.clone())?.finish(settings::Flags::new(flags))?;
    let mut module = ObjectModule::new(ObjectBuilder::new(
        isa,
        "ncc",
        default_libcall_names(),
    )?);
    let mut ctx = module.make_context();
    ctx.func.signature.returns.push(AbiParam::new(types::I32));
    ctx.func.name = UserFuncName::user(0, 0);
    let mut fb_ctx = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);
        let entry = b.create_block();
        b.switch_to_block(entry);
        b.seal_block(entry);
        let value = lower_expr(&function.return_value, &mut b)?;
        b.ins().return_(&[value]);
        b.finalize();
    }
    let id = module.declare_function(&function.name, Linkage::Export, &ctx.func.signature)?;
    module.define_function(id, &mut ctx)?;
    module.clear_context(&mut ctx);
    module.finish().emit().context("failed to emit object file")
}

fn lower_expr(expr: &Expr, b: &mut FunctionBuilder<'_>) -> Result<Value> {
    Ok(match expr {
        Expr::Integer(v) => {
            if i32::try_from(*v).is_err() {
                bail!("integer constant {v} is outside NCC's supported 32-bit int range");
            }
            b.ins().iconst(types::I32, *v)
        }
        Expr::Neg(x) if matches!(x.as_ref(), Expr::Integer(2_147_483_648)) => {
            b.ins().iconst(types::I32, i64::from(i32::MIN))
        }
        Expr::Neg(x) => { let v = lower_expr(x, b)?; b.ins().ineg(v) }
        Expr::Not(x) => {
            let v = lower_expr(x, b)?;
            let is_zero = b.ins().icmp_imm(IntCC::Equal, v, 0);
            b.ins().uextend(types::I32, is_zero)
        }
        Expr::BitNot(x) => { let v = lower_expr(x, b)?; b.ins().bnot(v) }
        Expr::Add(l, r) => bin(l, r, b, |b, l, r| b.ins().iadd(l, r))?,
        Expr::Sub(l, r) => bin(l, r, b, |b, l, r| b.ins().isub(l, r))?,
        Expr::Mul(l, r) => bin(l, r, b, |b, l, r| b.ins().imul(l, r))?,
        Expr::Div(l, r) => bin(l, r, b, |b, l, r| b.ins().sdiv(l, r))?,
        Expr::Rem(l, r) => bin(l, r, b, |b, l, r| b.ins().srem(l, r))?,
        Expr::Shl(l, r) => bin(l, r, b, |b, l, r| b.ins().ishl(l, r))?,
        Expr::Shr(l, r) => bin(l, r, b, |b, l, r| b.ins().sshr(l, r))?,
        Expr::Eq(l, r) => cmp(IntCC::Equal, l, r, b)?,
        Expr::Ne(l, r) => cmp(IntCC::NotEqual, l, r, b)?,
        Expr::Lt(l, r) => cmp(IntCC::SignedLessThan, l, r, b)?,
        Expr::Le(l, r) => cmp(IntCC::SignedLessThanOrEqual, l, r, b)?,
        Expr::Gt(l, r) => cmp(IntCC::SignedGreaterThan, l, r, b)?,
        Expr::Ge(l, r) => cmp(IntCC::SignedGreaterThanOrEqual, l, r, b)?,
        Expr::BitAnd(l, r) => bin(l, r, b, |b, l, r| b.ins().band(l, r))?,
        Expr::BitXor(l, r) => bin(l, r, b, |b, l, r| b.ins().bxor(l, r))?,
        Expr::BitOr(l, r) => bin(l, r, b, |b, l, r| b.ins().bor(l, r))?,
    })
}

fn bin<F: FnOnce(&mut FunctionBuilder<'_>, Value, Value) -> Value>(
    l: &Expr,
    r: &Expr,
    b: &mut FunctionBuilder<'_>,
    f: F,
) -> Result<Value> {
    let l = lower_expr(l, b)?;
    let r = lower_expr(r, b)?;
    Ok(f(b, l, r))
}

fn cmp(cc: IntCC, l: &Expr, r: &Expr, b: &mut FunctionBuilder<'_>) -> Result<Value> {
    let l = lower_expr(l, b)?;
    let r = lower_expr(r, b)?;
    let v = b.ins().icmp(cc, l, r);
    Ok(b.ins().uextend(types::I32, v))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_integer_constants_outside_supported_int_range() {
        let function = Function { name: "main".to_owned(), return_value: Expr::Integer(i64::from(i32::MAX) + 1) };
        let error = emit_object(&function).unwrap_err().to_string();
        assert!(error.contains("supported 32-bit int range"));
    }
    #[test]
    fn accepts_minimum_signed_32_bit_integer() {
        let function = Function { name: "main".to_owned(), return_value: Expr::Neg(Box::new(Expr::Integer(2_147_483_648))) };
        assert!(emit_object(&function).is_ok());
    }
    #[test]
    fn lowers_shift_expression() {
        let function = Function {
            name: "main".to_owned(),
            return_value: Expr::Shr(
                Box::new(Expr::Shl(Box::new(Expr::Integer(21)), Box::new(Expr::Integer(2)))),
                Box::new(Expr::Integer(1)),
            ),
        };
        assert!(emit_object(&function).is_ok());
    }
    #[test]
    fn lowers_binary_bitwise_expression() {
        let function = Function {
            name: "main".to_owned(),
            return_value: Expr::BitOr(
                Box::new(Expr::Integer(32)),
                Box::new(Expr::BitXor(Box::new(Expr::Integer(8)), Box::new(Expr::Integer(2)))),
            ),
        };
        assert!(emit_object(&function).is_ok());
    }
}
