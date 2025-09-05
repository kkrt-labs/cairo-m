(module
  (func $noop)

  (func $ret1 (param $a i32) (result i32)
    local.get $a
  )

  (func $call_noop)

  (func $call_ret1 (param $a i32) (result i32)
    local.get $a
    call $ret1
  )

  ;; Fused from func_call.wat: simple add and a main that calls it
  (func $add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add)

  (func $main (result i32)
    i32.const 15
    i32.const 25
    call $add)

  (memory 1)
  (export "memory" (memory 0))
  (export "call_noop" (func $call_noop))
  (export "call_ret1" (func $call_ret1))
  (export "main" (func $main))
)

