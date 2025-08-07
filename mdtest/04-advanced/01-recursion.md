# Recursion in Cairo-M

Recursive functions that call themselves to solve problems.

## Simple Recursion

Basic factorial implementation:

```cairo-m
fn test_factorial() -> felt {
    return factorial(5);  // Returns 120
}

fn factorial(n: felt) -> felt {
    if n == 0 {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

```

## Fibonacci Sequence

Classic recursive Fibonacci:

```cairo-m
fn test_fibonacci() -> felt {
    return fibonacci(7);  // Returns 13
}

fn fibonacci(n: felt) -> felt {
    if n == 0 {
        return 0;
    } else if n == 1 {
        return 1;
    } else {
        return fibonacci(n - 1) + fibonacci(n - 2);
    }
}

```
