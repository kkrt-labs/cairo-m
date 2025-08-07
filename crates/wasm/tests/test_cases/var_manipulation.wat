(module
  (type (;0;) (func (result i32)))
  (func $main (type 0) (result i32)
    (local $a i32)
    i32.const 10
    local.set $a
    local.get $a
    i32.const 5
    i32.add
    local.set $a
    local.get $a)
  (memory (;0;) 1)
  (export "main" (func $main))
  (export "memory" (memory 0))
) 
