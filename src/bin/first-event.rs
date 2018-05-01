extern crate dotenv;
extern crate marvel_explorer;
extern crate env_logger;
use std::env;
use dotenv::dotenv;
use marvel_explorer::MarvelClient;


fn main() {
    env_logger::init();
    dotenv().ok();

    let key = env::var("MARVEL_KEY").unwrap();
    let secret = env::var("MARVEL_SECRET_KEY").unwrap();

    let client = MarvelClient::new(key, secret);

    let name1: String = env::args().nth(1).unwrap();
    let name2: String = env::args().nth(2).unwrap();

    match client.earliest_event_match(&name1, &name2) {
        Err(e) => eprintln!("{:?}", e),
        Ok(maybe_event) => {
            println!("{:?}", maybe_event);
        }
    };
}
