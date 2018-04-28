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
use std::time::{SystemTime, UNIX_EPOCH};
use crypto::digest::Digest;
use crypto::md5::Md5;
use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;
use url::Url;

type HttpsClient = Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>;
type FutureJsonValue = Future<Item = serde_json::Value, Error = io::Error>;

#[derive(Debug, Deserialize)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct CharacterDataWrapper {
    pub data: Option<CharacterDataContainer>,
}

#[derive(Debug, Deserialize)]
struct CharacterDataContainer {
    pub offset: Option<i32>,
    pub limit: Option<i32>,
    pub total: Option<i32>,
    pub count: Option<i32>,
    pub results: Option<Vec<Character>>,
}

pub struct MarvelClient {
    key: String,
    secret: String,
    api_base: String,
    hasher: RefCell<Md5>,
    core: RefCell<Core>,
    http: HttpsClient,
}

impl MarvelClient {
    pub fn new(key: String, secret: String) -> MarvelClient {
        let core = Core::new().expect("new core");
        let handle = core.handle();
        let http = Client::configure()
            .connector(hyper_tls::HttpsConnector::new(4, &handle).unwrap())
            .build(&handle);
        MarvelClient {
            key,
            secret,
            api_base: "https://gateway.marvel.com:443/v1/public/".to_owned(),
            hasher: RefCell::new(Md5::new()),
            core: RefCell::new(core),
            http,
        }
    }

    fn get_hash(&self, ts: &str) -> String {
        let mut hasher = self.hasher.borrow_mut();
        hasher.reset();
        hasher.input_str(ts);
        hasher.input_str(&self.secret);
        hasher.input_str(&self.key);
        hasher.result_str()
    }

    fn build_url(&self, path: &str) -> Result<Url, url::ParseError> {
        let ts = {
            let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

            let ms = since_the_epoch.as_secs() * 1000
                + since_the_epoch.subsec_nanos() as u64 / 1_000_000;

            format!("{}", ms)
        };
        let hash = &self.get_hash(&ts);
        let mut url = Url::parse(&self.api_base)?.join(path)?;

        url.query_pairs_mut()
            .append_pair("ts", &ts)
            .append_pair("hash", hash)
            .append_pair("apikey", &self.key);

        Ok(url)
    }

    fn request_characters(&self, uri: hyper::Uri) -> Box<FutureJsonValue> {
        trace!("GET {}", uri);

        let f = self.http
            .get(uri)
            .and_then(|res| {
                trace!("Response: {}", res.status());
                res.body().concat2().and_then(move |body| {
                    let value: serde_json::Value = serde_json::from_slice(&body)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                    Ok(value)
                })
            })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e));

        Box::new(f)
    }

    pub fn search(&self, name_prefix: &str) -> Result<Vec<Character>, io::Error> {
        let mut url = self.build_url("characters").unwrap();
        url.query_pairs_mut()
            .append_pair("nameStartsWith", name_prefix);
        let uri = url.as_str()
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let work = self.request_characters(uri).and_then(|value| {
            let wrapper: CharacterDataWrapper =
                serde_json::from_value(value).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            Ok(wrapper
                .data
                .map(move |data: CharacterDataContainer| data.results.unwrap_or(vec![]))
                .unwrap_or(vec![]))
        });

        self.core.borrow_mut().run(work)
    }
}
