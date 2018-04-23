extern crate crypto;
extern crate hyper;
extern crate url;

use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;
use crypto::digest::Digest;
use crypto::md5::Md5;

pub struct Client {
    key: String,
    secret: String,
    api_base: String,
    hasher: RefCell<Md5>,
}

impl Client {
    pub fn new(key: String, secret: String) -> Client {
        Client {
            key,
            secret,
            api_base: "https://gateway.marvel.com:443/v1/public/".to_owned(),
            hasher: RefCell::new(Md5::new()),
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

    pub fn search(&self, name: &str) {
        let mut url = self.build_url("characters").unwrap();
        url.query_pairs_mut().append_pair("name", name);

        println!("{}", url.as_str());
        unimplemented!();
    }
}
