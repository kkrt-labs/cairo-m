(module
  (func $add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add)
  (func $func_call (result i32)
    i32.const 15
    i32.const 25
    call $add)
  (export "func_call" (func $func_call))
)
