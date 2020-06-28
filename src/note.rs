extern crate mailparse;
extern crate html2runes;

use std::borrow::Borrow;
use self::html2runes::markdown;

pub trait NoteTrait {
    fn subject(&self) -> String;
}

pub struct Note {
    pub mailHeaders: Vec<(String, String)>,
    pub body: String,
}

impl NoteTrait for Note {
    fn subject(&self) -> String {
        let subject = match self.mailHeaders.iter().find(|(x, y)| x.eq("Subject")) {
            Some((subject, name)) => name.to_owned(),
            _ => "<no subject>".to_string()
        };
        subject
    }
}