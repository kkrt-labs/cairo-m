(module $arithmetic-de93d964915565cf.wasm
 (type $i32_i32_=>_i32 (func (param i32 i32) (result i32)))
 (memory $0 16)
 (global $__stack_pointer (mut i32) (i32.const 1048576))
 (global $global$1 i32 (i32.const 1048576))
 (global $global$2 i32 (i32.const 1048576))
 (export "memory" (memory $0))
 (export "f" (func $f))
 (export "__data_end" (global $global$1))
 (export "__heap_base" (global $global$2))
 (func $f (param $0 i32) (param $1 i32) (result i32)
  (i32.mul
   (i32.add
    (local.get $1)
    (i32.const -3)
   )
   (i32.add
    (local.get $0)
    (i32.const 2)
   )
  )
 )
 ;; custom section "producers", size 59
)
