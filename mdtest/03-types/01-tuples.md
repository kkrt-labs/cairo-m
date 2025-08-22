# Tuples in Cairo-M

Tuples allow grouping multiple values of different types together into a single
compound type.

## Basic Tuple Creation

Simple tuple with mixed types:

//TODO: fix

```cairo-m
fn main_test() -> felt {
    let tuple = create_tuple();
    return tuple.0 + tuple.1 + tuple.2;
}

fn create_tuple() -> (felt, felt, felt) {
    return (1, 2, 3);
}
```

### Tuple with a single element

```cairo-m
fn create_tuple() -> (felt) {
    return (1);
}
```

## Tuple Destructuring

```cairo-m
fn destructuring_tuple() -> felt {
    let (a, b, c) = (1, 2, 3);
    return a + b + c;
}
```

## Tuple Member Access

```cairo-m
fn access_tuple_member() -> felt {
    let my_tuple = (1, 2, 3);
    return my_tuple.0 + my_tuple.1 + my_tuple.2;
}
```

## Tuple Member Assignment

```cairo-m
fn assign_tuple_member() -> felt {
    let my_tuple = (1, 2, 3);
    my_tuple.0 = 4;
    return my_tuple.0 + my_tuple.1 + my_tuple.2;
}
```

## Nested Tuples

```cairo-m
fn nested_tuples() -> felt {
    let tuple = (1, 2, (3, 4));
    return foo(tuple);
}

fn foo(input: (felt, felt, (felt, felt))) -> felt {
    let (a, b, (c, d)) = input;
    return a + b + c + d;
}
```
