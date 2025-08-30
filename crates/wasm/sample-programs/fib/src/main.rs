#![no_std]
#![no_main]

extern crate panic_abort;

#[unsafe(no_mangle)]
fn fib(n: i32) -> i32 {
    let mut a = 0;
    let mut b = 1;
    for _ in 1..n {
        let c = a + b;
        a = b;
        b = c;
    }
    b
}
