(module
  ;; Function to perform bitwise AND of two i32 values
  (func $and (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.and
  )

  ;; Function to perform bitwise OR of two i32 values
  (func $or (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.or
  )

  ;; Function to perform bitwise XOR of two i32 values
  (func $xor (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.xor
  )

  ;; Export functions for testing
  (export "and" (func $and))
  (export "or" (func $or))
  (export "xor" (func $xor))
  (memory (;0;) 1)
  (export "memory" (memory 0))
)
