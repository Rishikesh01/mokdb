use parser_v2::scanner::Scanner;

pub mod engine;
pub mod err;
mod parser_v2;
pub mod storage;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    Scanner::new("foo".to_string());
}
