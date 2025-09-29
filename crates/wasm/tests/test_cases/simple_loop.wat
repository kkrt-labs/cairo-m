(module
  (func $simple_loop (param $n i32) (result i32)
    (local $counter i32)
    (local $sum i32)

    ;; Initialize counter to 0
    i32.const 0
    local.set $counter

    ;; Initialize sum to 0
    i32.const 0
    local.set $sum

    ;; Loop: while counter < n
    loop $loop
      ;; Add counter to sum
      local.get $sum
      local.get $counter
      i32.add
      local.set $sum

      ;; Increment counter
      local.get $counter
      i32.const 1
      i32.add
      local.set $counter

      ;; Check if counter != n, continue loop if true
      local.get $counter
      local.get $n
      i32.lt_u
      br_if $loop
    end

    ;; Return sum (should be 0+1+2+...+n-1)
    local.get $sum
  )
  (export "simple_loop" (func $simple_loop))
)
