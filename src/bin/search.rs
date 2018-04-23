extern crate dotenv;
extern crate marvel_explorer;

use std::env;
use dotenv::dotenv;
use marvel_explorer::Client;

fn main() {
    dotenv().ok();

    let key = env::var("MARVEL_KEY").unwrap();
    let secret = env::var("MARVEL_SECRET_KEY").unwrap();

    let client = Client::new(key, secret);

    let name = env::args().nth(1).expect("name");

    client.search(&name);
}
