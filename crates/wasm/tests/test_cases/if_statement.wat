(module
  (func $if_statement (param $input i32) (result i32)
    (local $result i32)

    ;; Check if input equals 3
    local.get $input
    i32.const 3
    i32.eq

    if (result i32)
      ;; If input == 3, return 1
      i32.const 1
    else
      ;; If input != 3, check if input != 2
      local.get $input
      i32.const 2
      i32.ne

      if (result i32)
        ;; If input != 2, return input + 1
        local.get $input
        i32.const 1
        i32.add
      else
        ;; If input == 2, return 5
        i32.const 5
      end
    end
  )
  (export "if_statement" (func $if_statement))
)
