(module
  (func $locals (result i32)
    (local $a i32)
    i32.const 10
    local.set $a
    local.get $a
    i32.const 5
    i32.add
    local.set $a
    local.get $a)
  (export "locals" (func $locals))
)
