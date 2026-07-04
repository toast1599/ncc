use anyhow::{Context, Result};
use cranelift_codegen::ir::{AbiParam, InstBuilder, UserFuncName, types};
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;

use crate::frontend::Function;

pub fn emit_object(function: &Function) -> Result<Vec<u8>> {
    let triple = Triple::host();
    let mut flags = settings::builder();
    flags.set("is_pic", "true")?;
    let isa = isa::lookup(triple.clone())?.finish(settings::Flags::new(flags))?;
    let builder = ObjectBuilder::new(isa, "ncc", default_libcall_names())?;
    let mut module = ObjectModule::new(builder);

    let mut ctx = module.make_context();
    ctx.func.signature.returns.push(AbiParam::new(types::I32));
    ctx.func.name = UserFuncName::user(0, 0);

    let mut fb_ctx = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);
        let entry = b.create_block();
        b.switch_to_block(entry);
        b.seal_block(entry);
        let value = b.ins().iconst(types::I32, function.return_value);
        b.ins().return_(&[value]);
        b.finalize();
    }

    let id = module.declare_function(&function.name, Linkage::Export, &ctx.func.signature)?;
    module.define_function(id, &mut ctx)?;
    module.clear_context(&mut ctx);
    module.finalize_definitions()?;
    let product = module.finish();
    product.emit().context("failed to emit object file")
}
