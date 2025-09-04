(module
  ;; Mutable globals
  (global $g32 (mut i32) (i32.const 0))

(func $set_g32 (param $v i32)
  local.get $v
  global.set $g32
)

(func $get_g32 (result i32)
  global.get $g32
)

(func $main (param $a i32) (result i32)
  local.get $a
  call $set_g32
  call $get_g32
)

(memory 1)
(export "memory" (memory 0))
(export "main" (func $main))
(export "get_g32" (func $get_g32))
(export "set_g32" (func $set_g32))
)
