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

  (memory 1)
  (export "memory" (memory 0))
  (export "call_noop" (func $call_noop))
  (export "call_ret1" (func $call_ret1))
)

