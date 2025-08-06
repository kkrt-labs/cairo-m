use crate::loader::{WasmLoadError, WasmModule};
use cairo_m_compiler_mir::{module::MirModule, FunctionId, MirFunction};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmModuleToMirError {
    #[error("Failed to load Wasm module: {0}")]
    WasmLoadError(#[from] WasmLoadError),
}

pub struct WasmModuleToMir {
    module: WasmModule,
}

impl WasmModuleToMir {
    pub const fn new(module: WasmModule) -> Self {
        Self { module }
    }

    fn function_to_mir(&self, func_idx: &u32) -> Result<MirFunction, WasmModuleToMirError> {
        let _func = self
            .module
            .with_program(|program| program.functions.get(func_idx).unwrap());

        let func_name = self.module.with_program(|program| {
            program
                .c
                .exported_functions
                .get(func_idx)
                .unwrap()
                .to_string()
        });
        let mir_function = MirFunction::new(func_name);
        // TODO: Implement the flattening logic
        Ok(mir_function)
    }

    pub fn to_mir(&self) -> Result<MirModule, WasmModuleToMirError> {
        let mut mir_module = MirModule::new();
        self.module.with_program(|program| {
            for (func_idx, _) in program.functions.iter() {
                let function_id = FunctionId::new(*func_idx as usize);
                let mir_function = self.function_to_mir(func_idx).unwrap();
                mir_module
                    .function_names
                    .insert(mir_function.name.clone(), function_id);
                mir_module.functions.insert(function_id, mir_function);
            }
        });
        Ok(mir_module)
    }
}
