extern crate alloc;
extern crate core;

mod paging;
mod types;
mod errors;

fn main() {
    println!("Hello, world!");
    let _ = paging::Page::new_inner();
}
