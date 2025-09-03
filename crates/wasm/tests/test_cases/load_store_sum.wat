(module
  (type (;0;) (func (param i32) (result i32)))
  (func $load_store_sum (type 0) (param $n i32) (result i32)
    (local $i i32)
    (local $sum i32)
    (local $address i32)

    ;; Initialize variables
    i32.const 1
    local.set $i

    i32.const 0
    local.set $sum

    i32.const 0  ;; Start storing at memory address 0
    local.set $address

    ;; First loop: store numbers 1 to n in memory
    loop $store_loop
      ;; Store current number i at address
      local.get $address
      local.get $i
      i32.store

      ;; Increment address by 4 (size of i32)
      local.get $address
      i32.const 4
      i32.add
      local.set $address

      ;; Increment i
      local.get $i
      i32.const 1
      i32.add
      local.set $i

      ;; Continue loop if i <= n
      local.get $i
      local.get $n
      i32.le_u
      br_if $store_loop
    end

    ;; Reset variables for second loop
    i32.const 0
    local.set $address

    i32.const 1
    local.set $i

    ;; Second loop: load numbers from memory and sum them
    loop $load_loop
      ;; Load value from current address and add to sum
      local.get $sum
      local.get $address
      i32.load
      i32.add
      local.set $sum

      ;; Increment address by 4
      local.get $address
      i32.const 4
      i32.add
      local.set $address

      ;; Increment i
      local.get $i
      i32.const 1
      i32.add
      local.set $i

      ;; Continue loop if i <= n
      local.get $i
      local.get $n
      i32.le_u
      br_if $load_loop
    end

    ;; Return the sum
    local.get $sum
  )
  (memory (;0;) 1)  ;; 1 page = 64KB of memory
  (export "load_store_sum" (func $load_store_sum))
  (export "memory" (memory 0))
)
