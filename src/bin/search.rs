extern crate dotenv;
extern crate marvel_explorer;

use std::env;
use dotenv::dotenv;
use marvel_explorer::MarvelClient;

fn main() {
    dotenv().ok();

    let key = env::var("MARVEL_KEY").unwrap();
    let secret = env::var("MARVEL_SECRET_KEY").unwrap();

    let client = MarvelClient::new(key, secret);

    let name = env::args().nth(1).expect("name");

    let _ = client.search(&name).unwrap();
}
