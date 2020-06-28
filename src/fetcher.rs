extern crate imap;
extern crate native_tls;
extern crate mailparse;
extern crate log;
extern crate regex;


use std::fs;
use std::fs::File;
use self::log::{info, trace, warn, debug};
use std::io::Write;

struct Collector(Vec<u8>);

use self::imap::Session;
use std::net::TcpStream;
use self::native_tls::TlsStream;
use self::imap::types::{ZeroCopy, Fetch};
use std::ptr::null;
use std::borrow::Borrow;
use std::any::Any;
use self::regex::Regex;
use note::{Note, NoteTrait};
use fetcher;
use converter;

pub trait MailFetcher {
    fn fetchMails() -> Vec<Note>;
}


pub fn login() -> Session<TlsStream<TcpStream>> {
    let creds = fs::read_to_string("cred").expect("error");

    let username_regex = Regex::new(r"^username=(.*)").unwrap();
    let password_regex = Regex::new(r"password=(.*)").unwrap();

    let username = username_regex.captures(creds.as_str()).unwrap().get(1).unwrap().as_str();
    let password = password_regex.captures(creds.as_str()).unwrap().get(1).unwrap().as_str();

    let domain = "imap.ankaa.uberspace.de";
    let tls = native_tls::TlsConnector::builder().danger_accept_invalid_certs(true).build().unwrap();

    // we pass in the domain twice to check that the server's TLS
    // certificate is valid for the domain we're connecting to.
    let client = imap::connect((domain, 993), domain, &tls).unwrap();

    // the client we have here is unauthenticated.
    // to do anything useful with the e-mails, we need to log in
    let imap_session = client
        .login(username, password)
        .map_err(|e| e.0);

    return imap_session.unwrap();
}

pub fn fetch_inbox_top() -> imap::error::Result<Option<String>> {
    let mut imap_session = login();

    // we want to fetch the first email in the INBOX mailbox

    let _count =
        imap_session.list(None, None).iter().next().iter().count();

    let _mailbox = imap_session.examine("Notes").unwrap();

    // fetch message number 1 in this mailbox, along with its RFC822 field.
    // RFC 822 dictates the format of the body of e-mails
    let messages = imap_session.fetch("1:*", "RFC822.HEADER")?;

    let folders = imap_session.list(None, None);

    info!("Folder count: {}", folders.iter().count());

    folders.iter().for_each(|folder| {
        folder.iter().for_each(|d| {
            info!("Folder names: {}", d.name().to_string());
        })
    });

    let iterator = messages.iter();

    iterator.for_each(|message| {
        let subject_rgex = Regex::new(r"Subject:(.*)").unwrap();

        // extract the message's body
        let header = message.header().expect("message did not have a body!");
        let header = std::str::from_utf8(header).expect("message was not valid utf-8").to_string();
        let _subject = subject_rgex.captures(header.as_str()).unwrap().get(1).unwrap().as_str();
        // println!("{}", header);
    });


    // be nice to the server and log out
    imap_session.logout()?;

    Ok(Some("ddd".to_string()))
}

pub fn get_messages_from_foldersession(session: &mut Session<TlsStream<TcpStream>>, folderName: String) -> Vec<Note> {
    session.select(&folderName);
    let messages_result = session.fetch("1:*", "(RFC822 RFC822.HEADER)");
    let messages = match messages_result {
        Ok(messages) => {
            debug!("Message Loading for {} successful", &folderName.to_string());
            get_notes(messages)
        }
        Err(error) => {
            warn!("Could not load notes from {}!", &folderName.to_string());
            Vec::new()
        }
    };
    messages
}

pub fn get_notes(fetch_vector: ZeroCopy<Vec<Fetch>>) -> Vec<Note> {
    fetch_vector.into_iter().map(|fetch| {
        let headers = get_headers(fetch.borrow());
        let body = get_body(fetch.borrow());
        Note {
            mailHeaders: headers,
            body: body.unwrap_or("mist".to_string()),
        }
    }).collect()
}

/**
Returns empty vector if something fails
*/
pub fn get_headers(fetch: &Fetch) -> Vec<(String, String)> {
    match mailparse::parse_headers(fetch.header().unwrap()) {
        Ok((header, _)) => header.into_iter().map(|h| (h.get_key().unwrap(), h.get_value().unwrap())).collect(),
        _ => Vec::new()
    }
}

pub fn get_body(fetch: &Fetch) -> Option<String> {
    match mailparse::parse_mail(fetch.body()?) {
        Ok(body) => body.get_body().ok(),
        _ => None
    }
}

pub fn list_note_folders(imap: &mut Session<TlsStream<TcpStream>>) -> Vec<String> {
    let folders_result = imap.list(None, Some("Notes*"));
    let result: Vec<String> = match folders_result {
        Ok(result) => {
            let names: Vec<String> = result.iter().map(|name| name.name().to_string()).collect();
            names
        }
        _ => Vec::new()
    };

    return result;
}

pub fn save_all_notes_to_file(session: &mut Session<TlsStream<TcpStream>>) {
    let folders = list_note_folders(session);

    folders.iter().for_each(|folderName| {
        let _messages = fetcher::get_messages_from_foldersession(session, folderName.to_string());

        _messages.iter().for_each(|note| {
            let location = "/home/findus/.notes/".to_string() + folderName + "/" + &note.subject().replace("/", "_");
            info!("Save to {}", location);

            let path = std::path::Path::new(&location);
            let prefix = path.parent().unwrap();
            std::fs::create_dir_all(prefix).unwrap();

            let mut f = File::create(location).expect("Unable to create file");
            f.write_all(converter::convert2md(&note.body).as_bytes()).expect("Unable to write file")
        });
    });
}

#[cfg(test)]
mod tests {
    //mod notes;
    //use imap;
    use crate::notes::*;
    use crate::mail::fetcher::*;
    use notes::note::NoteTrait;
    use fetcher::save_all_notes_to_file;


    #[test]
    fn login() {
        simple_logger::init().unwrap();
        let mut session = crate::mail::fetcher::login();
        save_all_notes_to_file(&mut session);
    }
}

