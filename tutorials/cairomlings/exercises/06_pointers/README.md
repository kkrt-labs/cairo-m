# Pointers

Cairo-M supports allocating memory on a different region of memory than the one used by local variables; which can be used to store data and reference it by pointer.

Typically, this is useful when you want to work with dynamically-sized collection of data, or when you want to pass around data by reference.

To allocate memory in that secondary region, you can use the `new` keyword.
You can then access and mutate the allocated memory via pointer indexing.
