extern crate dotenv;
extern crate marvel_explorer;
#[macro_use]
extern crate prettytable;

use std::env;
use dotenv::dotenv;
use prettytable::Table;
use prettytable::format;
use marvel_explorer::MarvelClient;

fn main() {
    dotenv().ok();

    let key = env::var("MARVEL_KEY").unwrap();
    let secret = env::var("MARVEL_SECRET_KEY").unwrap();

    let client = MarvelClient::new(key, secret);

    let name1 = env::args().nth(1).expect("name1");
    let name2 = env::args().nth(2).expect("name1");
}
