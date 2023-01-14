use rocust::rocust_lib;
use rocust::rocust_macros;

#[rocust_macros::add_field]
struct MyStruct {
    pub a: i32,
    pub b: i32,
}

fn main() {
    println!("Hello, world!");
}
