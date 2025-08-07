(module $if_statement-3b6e251b8cf7b879.wasm
 (type $i32_=>_i32 (func (param i32) (result i32)))
 (memory $0 16)
 (global $__stack_pointer (mut i32) (i32.const 1048576))
 (global $global$1 i32 (i32.const 1048576))
 (global $global$2 i32 (i32.const 1048576))
 (export "memory" (memory $0))
 (export "f" (func $f))
 (export "__data_end" (global $global$1))
 (export "__heap_base" (global $global$2))
 (func $f (param $0 i32) (result i32)
  (select
   (i32.const 1)
   (i32.shl
    (i32.ne
     (local.get $0)
     (i32.const 2)
    )
    (i32.const 1)
   )
   (i32.eq
    (local.get $0)
    (i32.const 3)
   )
  )
 )
 ;; custom section "producers", size 59
)
