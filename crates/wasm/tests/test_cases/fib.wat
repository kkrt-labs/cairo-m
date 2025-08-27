(module
  (func $fib (param $n i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local $temp i32)
    (local $i i32)

    ;; if n <= 1, return n
    (local.get $n)
    (i32.const 1)
    (i32.le_u)
    (if
      (then
        (local.get $n)
        (return)
      )
    )

    ;; else, compute fibonacci iteratively
    (i32.const 0)  ;; a = 0
    (local.set $a)
    (i32.const 1)  ;; b = 1
    (local.set $b)
    (i32.const 2)  ;; i = 2
    (local.set $i)

    ;; loop: while i <= n
    (block $loop
      (loop $loop_body
        ;; if i > n, break
        (local.get $i)
        (local.get $n)
        (i32.gt_u)
        (br_if $loop)

        ;; temp = a + b
        (local.get $a)
        (local.get $b)
        (i32.add)
        (local.set $temp)

        ;; a = b
        (local.get $b)
        (local.set $a)

        ;; b = temp
        (local.get $temp)
        (local.set $b)

        ;; i = i + 1
        (local.get $i)
        (i32.const 1)
        (i32.add)
        (local.set $i)

        ;; continue loop
        (br $loop_body)
      )
    )

    ;; return b
    (local.get $b)
  )

  (export "fib" (func $fib))
)
