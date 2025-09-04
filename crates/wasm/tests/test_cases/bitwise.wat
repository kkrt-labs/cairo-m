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

  ;; Function to perform bitwise shift left of two i32 values
  (func $shl (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.shl
  )

  ;; Function to perform bitwise shift right of two i32 values
  (func $shr_u (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.shr_u
  )

  ;; Function to perform bitwise rotate left of two i32 values
  (func $rotl (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.rotl
  )

  ;; Function to perform bitwise rotate right of two i32 values
  (func $rotr (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.rotr
  )

  ;; Export functions for testing
  (export "and" (func $and))
  (export "or" (func $or))
  (export "xor" (func $xor))
  (export "shl" (func $shl))
  (export "shr_u" (func $shr_u))
  (export "rotl" (func $rotl))
  (export "rotr" (func $rotr))
  (memory (;0;) 1)
  (export "memory" (memory 0))
)
