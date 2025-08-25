(module
  (type (;0;) (func (param i32 i32) (result i32)))
  (type (;1;) (func (result i32)))
  (func $add (type 0) (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add)
  (func $main (type 1) (result i32)
    i32.const 15
    i32.const 25
    call $add)
  (memory (;0;) 1)
  (export "main" (func $main))
  (export "memory" (memory 0))
) 
