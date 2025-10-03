(module
  ;; Simple add function with memory interaction:
  ;; stores a at address 0 and b at address 4, then loads both and returns their sum
  (func $add (param $a i32) (param $b i32) (result i32)
    ;; store a at address 0
    i32.const 0
    local.get $a
    i32.store
    ;; store b at address 4
    i32.const 4
    local.get $b
    i32.store
    ;; load a from address 0
    i32.const 0
    i32.load
    ;; load b from address 4 and add
    i32.const 4
    i32.load
    i32.add
  )



  ;; Store numbers 1 to n in memory and then load them and sum them
  ;; This checks that the store and load work as expected and that there are no collisions between the u32 values
  (func $load_store_sum (param $n i32) (result i32)
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

  (func $load_store_sum_3_with_offsets (result i32)
    i32.const 0
    i32.const 65000 ;; values larger than u16_max to check for any overlaps
    i32.store offset=0
    i32.const 0
    i32.const 66000
    i32.store offset=4
    i32.const 0
    i32.const 67000
    i32.store offset=8
    i32.const 0
    i32.load offset=0
    i32.const 0
    i32.load offset=4
    i32.const 0
    i32.load offset=8
    i32.add
    i32.add
  )


  (memory (;0;) 1)  ;; 1 page = 64KB of memory
  (export "load_store_sum" (func $load_store_sum))
  (export "add" (func $add))
  (export "load_store_sum_3_with_offsets" (func $load_store_sum_3_with_offsets))
  (export "memory" (memory 0))
)
