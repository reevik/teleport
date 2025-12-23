mod paging;

fn main() {
    println!("Hello, world!");
    let page = paging::SlottedPage::new_inner();
}
