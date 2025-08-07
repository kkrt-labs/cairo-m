# Tuples in Cairo-M

Tuples allow grouping multiple values of different types together into a single
compound type.

## Basic Tuple Creation

Simple tuple with mixed types:

//TODO: fix

```cairo-m
//! ignore: true
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
