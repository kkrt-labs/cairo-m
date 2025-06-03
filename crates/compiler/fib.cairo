func main(){
   return fib(7, 0, 1);
}

func fib(n, a, b) {
    if (n == 0) {
        return a;
    }
    if (n == 1) {
        return b;
    }
    return fib(n - 1, b, a + b);
}