#![no_std]
#![no_main]

extern crate panic_abort;

#[unsafe(no_mangle)]
fn ackermann(m: i32, n: i32) -> i32 {
    if m == 0 {
        n + 1
    } else if n == 0 {
        ackermann(m - 1, 1)
    } else {
        ackermann(m - 1, ackermann(m, n - 1))
    }
}
