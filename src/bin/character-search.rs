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

    let name = env::args().nth(1).expect("name");

    match client.search_characters(&name) {
        Err(e) => eprintln!("{:?}", e),
        Ok(results) => {
            // Create the table
            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            // Add a row
            table.set_titles(row!["ID", "Name", "Description"]);

            for character in &results {
                // TODO: come up with a clever way to add newlines at an interval to wrap long text.
                let description = character.description.chars().take(60).collect::<String>();
                table.add_row(row![character.id, character.name, description]);
            }
            table.printstd();
        }
    };
}
