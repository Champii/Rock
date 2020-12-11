use crate::codegen::IrBuilder;
use crate::codegen::IrContext;

use llvm_sys::core::LLVMArrayType;
use llvm_sys::core::LLVMBuildAlloca;
use llvm_sys::core::LLVMBuildBitCast;
use llvm_sys::core::LLVMBuildGEP;
use llvm_sys::core::LLVMBuildStore;
use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMGetElementType;
use llvm_sys::core::LLVMInt32Type;
use llvm_sys::core::LLVMInt8Type;
use llvm_sys::core::LLVMPointerType;
use llvm_sys::LLVMValue;

impl IrBuilder for String {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        let mut me = self.clone();

        me.push(0 as char);

        unsafe {
            let t = LLVMPointerType(LLVMInt8Type(), 0);

            let pointer = LLVMBuildAlloca(
                context.builder,
                LLVMArrayType(LLVMGetElementType(t), me.len() as u32),
                b"\0".as_ptr() as *const _,
            );

            let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);
            let mut indices = [zero, zero];

            let ptr_elem = LLVMBuildGEP(
                context.builder,
                pointer,
                indices.as_mut_ptr(),
                2,
                b"\0".as_ptr() as *const _,
            );

            let mut i = 0;

            for item in me.bytes() {
                let idx = LLVMConstInt(LLVMInt32Type(), i, 0);
                let mut indices = [zero, idx];

                let ptr_elem = LLVMBuildGEP(
                    context.builder,
                    pointer,
                    indices.as_mut_ptr(),
                    2,
                    b"\0".as_ptr() as *const _,
                );

                let idx = LLVMConstInt(LLVMInt8Type(), item as u64, 0);

                LLVMBuildStore(context.builder, idx, ptr_elem);

                i += 1;
            }

            let ptr8 = LLVMBuildBitCast(context.builder, ptr_elem, t, b"\0".as_ptr() as *const _);

            Some(ptr8)
        }
    }
}
