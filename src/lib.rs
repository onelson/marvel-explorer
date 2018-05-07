extern crate crypto;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate url;

use std::cell::RefCell;
use std::io;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use crypto::digest::Digest;
use crypto::md5::Md5;
use futures::{Future, Stream};
use hyper::{Client, Uri};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use url::Url;

type HttpsClient = Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>;
type FutureJsonValue = Future<Item = serde_json::Value, Error = io::Error>;

#[derive(Debug, Deserialize)]
pub struct PaginationDetails {
    pub offset: i32,
    pub limit: i32,
    pub total: i32,
    pub count: i32,
}

#[derive(Debug, Deserialize)]
pub struct Character {
    pub id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Event {
    pub id: i32,
    pub title: String,
    pub start: Option<String>,
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct DataWrapper<T> {
    pub data: DataContainer<T>,
}

#[derive(Debug, Deserialize)]
struct DataContainer<T> {
    #[serde(flatten)]
    pub page_details: PaginationDetails,
    pub results: Vec<T>,
}

fn to_io_error<E>(err: E) -> io::Error
where
    E: Into<Box<std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::Other, err)
}

const MAX_LIMIT: usize = 100;

struct UriMaker {
    key: String,
    secret: String,
    api_base: String,
    hasher: RefCell<Md5>,
}

impl UriMaker {
    pub fn new(key: String, secret: String, api_base: String) -> UriMaker {
        UriMaker {
            key,
            secret,
            api_base,
            hasher: RefCell::new(Md5::new()),
        }
    }

    /// The Marvel API authorization scheme requires we produce a hash of our public and
    /// private keys, in addition to a trace value (a timestamp) which must also be sent in clear
    /// text as the `ts` parameter (so they can verify our shared secret).
    fn get_hash(&self, ts: &str) -> String {
        let mut hasher = self.hasher.borrow_mut();
        hasher.reset();
        hasher.input_str(ts);
        hasher.input_str(&self.secret);
        hasher.input_str(&self.key);
        hasher.result_str()
    }

    /// Convert from a `url::Url` to a `hyper::Uri`.
    fn url_to_uri(url: &url::Url) -> Uri {
        url.as_str().parse().unwrap()
    }

    /// Append a path to the api root, as well as the authorization query string params.
    fn build_url(&self, path: &str) -> Result<Url, url::ParseError> {
        let ts = {
            let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

            let ms = since_the_epoch.as_secs() * 1000
                + since_the_epoch.subsec_nanos() as u64 / 1_000_000;

            format!("{}", ms)
        };
        let hash = self.get_hash(&ts);
        let mut url = Url::parse(&self.api_base)?.join(path)?;

        url.query_pairs_mut()
            .append_pair("ts", &ts)
            .append_pair("hash", &hash)
            .append_pair("apikey", &self.key);

        Ok(url)
    }

    /// Lookup character data by name (exact match).
    pub fn character_by_name_exact(&self, name: &str) -> Uri {
        let mut url = self.build_url("characters").unwrap();
        url.query_pairs_mut().append_pair("name", name);
        Self::url_to_uri(&url)
    }

    /// Lookup character data by name (using a "starts with" match).
    pub fn character_by_name(&self, name_starts_with: &str) -> Uri {
        let mut url = self.build_url("characters").unwrap();
        url.query_pairs_mut()
            .append_pair("nameStartsWith", name_starts_with);
        Self::url_to_uri(&url)
    }

    /// Get all the events for a given character.
    ///
    /// At the time of writing, there are only around 75 events in the database, meaning we should
    /// not have to page through the data ever. Setting the limit to the max (currently 100) should
    /// mean each request made this way should include the full set of events for that character.
    pub fn character_events(&self, character_id: i32) -> Uri {
        let mut url = self.build_url(&format!("characters/{}/events", character_id))
            .unwrap();
        url.query_pairs_mut()
            .append_pair("limit", &format!("{}", MAX_LIMIT));
        Self::url_to_uri(&url)
    }
}

/// The top level interface for interacting with the remote service.
pub struct MarvelClient {
    /// provides the means to generate uris with correct authorization info attached.
    uri_maker: UriMaker,
    /// tokio core to run our requests in.
    core: RefCell<Core>,
    /// hyper http client to build requests with.
    http: HttpsClient,
}

impl MarvelClient {
    pub fn new(key: String, secret: String) -> MarvelClient {
        let core = Core::new().unwrap();

        let http = {
            let handle = core.handle();
            let connector = HttpsConnector::new(4, &handle).unwrap();

            Client::configure()
                .connector(connector)
                .build(&handle)
        };

        let uri_maker = UriMaker::new(
            key,
            secret,
            "https://gateway.marvel.com:443/v1/public/".to_owned(),
        );

        MarvelClient {
            uri_maker,
            core: RefCell::new(core),
            http,
        }
    }

    /// Given a uri to access, this generates a future json value (to be executed by a core later).
    fn get_json(&self, uri: hyper::Uri) -> Box<FutureJsonValue> {
        debug!("GET {}", uri);

        let f = self.http
            .get(uri)
            .and_then(|res| {
                debug!("Response: {}", res.status());
                res.body().concat2().and_then(move |body| {
                    let value: serde_json::Value =
                        serde_json::from_slice(&body).map_err(to_io_error)?;

                    Ok(value)
                })
            })
            .map_err(to_io_error);

        Box::new(f)
    }

    pub fn search_characters(&self, name_prefix: &str) -> Result<Vec<Character>, io::Error> {
        let uri = self.uri_maker.character_by_name(name_prefix);
        let work = self.get_json(uri).and_then(|value| {
            let wrapper: DataWrapper<Character> =
                serde_json::from_value(value).map_err(to_io_error)?;

            Ok(wrapper.data.results)
        });

        self.core.borrow_mut().run(work)
    }

    pub fn events_by_character(&self, character_id: i32) -> Result<Vec<Event>, io::Error> {
        let uri = self.uri_maker.character_events(character_id);
        let work = self.get_json(uri).and_then(|value| {
            let wrapper: DataWrapper<Event> = serde_json::from_value(value).map_err(to_io_error)?;

            Ok(wrapper.data.results)
        });

        self.core.borrow_mut().run(work)
    }

    pub fn earliest_event_match(
        &self,
        name1: &str,
        name2: &str,
    ) -> Result<Option<Event>, io::Error> {
        // While possible to have this closure be an actual function, you'll likely begin to run
        // into lifetime issues around the lifetimes of args to each subsequent hop in the chain.
        // The borrow checker seems to be satisfied if this all happens within the same scope where
        // the futures are sent into the core for execution, however.
        let name_to_event_set = |name: String| {
            let id_lookup = self.uri_maker.character_by_name_exact(&name);
            self.get_json(id_lookup)
                .and_then(move |characters_resp| {
                    // In this closure, we're returning a Result to factor in the potential
                    // json parse failure, or the fact that maybe the exact name doesn't exist in
                    // the database.
                    // The `move` is required on this closure to keep `name` alive so we can give
                    // nice error output showing which name lookup failed.
                    let wrapper: DataWrapper<Character> =
                        serde_json::from_value(characters_resp).map_err(to_io_error)?;

                    match wrapper.data.results.first() {
                        Some(character) => Ok(character.id),
                        None => Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Character `{}` Not Found", name),
                        )),
                    }
                })
                .and_then(|id| {
                    let uri = self.uri_maker.character_events(id);
                    // return a future from a future and the next link in the chain will wait until
                    // this inner-future resolves
                    self.get_json(uri)
                })
                .and_then(|events_resp| { // response from the call to `self.get_json()` above.
                    let wrapper: DataWrapper<Event> =
                        serde_json::from_value(events_resp).map_err(to_io_error)?;
                    // Using `into_iter()` here allows us to "move" the `Event` instances
                    // from the Vec into the `HashSet` without copying.
                    let result_set: HashSet<Event> = wrapper.data.results.into_iter().collect();
                    Ok(result_set)
                })
        };

        // In this case, `work` is a graph of futures:
        // * The pipeline defined in the closure above (`name_to_event_set`) runs twice in parallel
        // * When both pipelines complete, the resolved values are then passed into the `and_then`
        //   continuation where we compute the intersection of the two sets for our final result.
        let work = name_to_event_set(name1.to_owned())
            .join(name_to_event_set(name2.to_owned()))
            .and_then(|(events1, events2)| {
                let maybe_event: Option<Event> = events1.intersection(&events2)
                    .min_by_key(|x| &x.start)
                    .map(|x| x.clone());
                Ok(maybe_event)
            });

        self.core.borrow_mut().run(work)
    }
}
