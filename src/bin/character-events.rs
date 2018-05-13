extern crate dotenv;
extern crate marvel_explorer;
#[macro_use]
extern crate prettytable;

use dotenv::dotenv;
use marvel_explorer::MarvelClient;
use prettytable::Table;
use prettytable::format;
use std::env;

fn format_description(event: &marvel_explorer::Event) -> String {
    // TODO: come up with a clever way to add newlines at an interval to wrap long text.
    event.description.chars().take(40).collect::<String>()
}

fn main() {
    dotenv().ok();

    let key = env::var("MARVEL_KEY").unwrap();
    let secret = env::var("MARVEL_SECRET_KEY").unwrap();

    let client = MarvelClient::new(key, secret);

    let id: i32 = env::args()
        .nth(1)
        .expect("character_id")
        .parse()
        .expect("parse character_id");

    match client.events_by_character(id) {
        Err(e) => eprintln!("{:?}", e),
        Ok(results) => {
            // Create the table
            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            // Add a row
            table.set_titles(row!["ID", "Title", "Date", "Description"]);

            for event in &results {
                let description = format_description(&event);

                let start = match event.start {
                    Some(ref s) => s,
                    None => "",
                };
                table.add_row(row![event.id, event.title, start, description]);
            }
            table.printstd();
        }
    };
}
