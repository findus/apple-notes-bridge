use model::{Body, NotesMetadata};
use util::generate_uuid;
use note::{RemoteNoteMetaData, NoteHeaders, HeaderParser};
use chrono::Utc;
use profile;

pub struct NoteTupleBuilder {
    metadata: NotesMetadata,
    body: Body
}

impl NoteTupleBuilder {

}

pub struct BodyMetadataBuilder {
    body: Body
}
/// Builder for Body Objects, mostly for
/// testing purposes
///
/// If no own message-id gets provided it gets randomly
/// generated
impl BodyMetadataBuilder {
    pub fn new() -> BodyMetadataBuilder {
        let profile = self::profile::load_profile();
        BodyMetadataBuilder {
            body: Body {
                message_id: format!("<{}@{}", generate_uuid(), profile.domain()),
                text: None,
                uid: Some(-1),
                metadata_uuid: "".to_string()
            }
        }
    }

    pub fn with_uid(mut self, uid: i64) -> Self {
        self.body.uid = Some(uid);
        self
    }

    pub fn with_metadata_uuid(mut self, uuid: &str) -> Self {
        let profile = self::profile::load_profile();
        self.body.metadata_uuid =  format!("<{}@{}", uuid.to_string(), profile.domain());
        self
    }

    pub fn with_message_id(mut self, message_id: &str) -> Self {
        self.body.message_id = message_id.to_string();
        self
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.body.text = Some(text.to_string());
        self
    }

    pub fn build(self) -> Body {
        self.body
    }
}

pub struct NotesMetadataBuilder {
    notesMetadata: NotesMetadata
}

/// Builder for Metadata Objects, mostly for
/// testing purposes
///
/// If no own uuid gets provided it gets randomly
/// generated
impl NotesMetadataBuilder {
    pub fn new() -> NotesMetadataBuilder {
        let date = Utc::now().to_rfc2822();
        NotesMetadataBuilder {
            notesMetadata:  NotesMetadata {
                old_remote_id: None,
                subfolder: "".to_string(),
                locally_deleted: false,
                locally_edited: false,
                new: false,
                date,
                uuid: generate_uuid(),
                mime_version: "1.0 (Mac OS X Notes 4.6 \\(879.10\\))".to_string()
            }
        }

    }

    pub fn with_uuid(mut self, uuid: String) -> Self {
        self.notesMetadata.uuid = uuid;
        self
    }

    pub fn is_new(mut self, new: bool) -> Self {
        self.notesMetadata.new = new;
        self
    }

    pub fn is_flagged_for_deletion(mut self, del: bool) -> Self {
        self.notesMetadata.locally_deleted = del;
        self
    }

    pub fn with_folder(mut self, folder: String) -> Self {
        self.notesMetadata.subfolder = folder;
        self
    }

    pub fn build(self) -> NotesMetadata {
        self.notesMetadata
    }
    
    pub fn build_as_remote_data(self) -> RemoteNoteMetaData {
        RemoteNoteMetaData {
            headers: vec![],
            folder: "".to_string(),
            uid: 0
        }
    }
}

pub struct HeaderBuilder {
    headers: Vec<(String,String)>
}

impl HeaderBuilder {

    pub fn new() -> HeaderBuilder {
        let mut headers: Vec<(String,String)> = vec![];
        let profile = profile::load_profile();
        headers.push(("X-Uniform-Type-Identifier".to_string(), "com.apple.mail-note".to_string()));
        headers.push(("Content-Type".to_string(), "text/html; charset=utf-8".to_string()));
        headers.push(("Content-Transfer-Encoding".to_string(), "quoted-printable".to_string()));
        headers.push(("Mime-Version".to_string(), "1.0 (Mac OS X Notes 4.6 \\(879.10\\))".to_string()));
        let date = Utc::now().to_rfc2822();
        headers.push(("Date".to_string(), date.clone()));
        headers.push(("X-Mail-Created-Date".to_string(), date.clone()));
        headers.push(("From".to_string(), profile.email));

        HeaderBuilder {
            headers
        }
    }

    pub fn with_message_id(mut self, message_id: String) -> Self {
        let profile = self::profile::load_profile();
        self.headers.push(("Message-Id".to_string(), format!("<{}@{}", message_id, profile.domain())));
        self
    }

    pub fn with_uuid(mut self, uuid: String) -> Self {
        self.headers.push(("X-Universally-Unique-Identifier".to_string(), uuid));
        self
    }

    pub fn with_subject(mut self, subject: &str) -> Self {
        self.headers.push(("Subject".to_string(), subject.to_string()));
        self
    }

    pub fn build(mut self) -> NoteHeaders {
        let profile = self::profile::load_profile();

        if None == self.headers.get_header_value("X-Universally-Unique-Identifier") {
            self.headers.push(("X-Universally-Unique-Identifier".to_string(), generate_uuid()));
        }

        if None == self.headers.get_header_value("Message-Id") {
            self.headers.push(("Message-Id".to_string(), format!("<{}@{}", generate_uuid(), profile.domain())));
        }

        self.headers
    }
}
