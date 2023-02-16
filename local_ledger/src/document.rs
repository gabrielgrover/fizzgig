use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::{Read, Seek, Write},
    path::PathBuf,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::utility::{generate_id, LocalLedgerError};

const DOCUMENT_CONFLICT_THRESHOLD: i64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document<T> {
    label: String,
    uuid: String,
    rev: String,
    data: T,
    seq: i64,
}

impl<T> Document<T>
where
    T: Clone + Serialize + DeserializeOwned + Default,
{
    /// Creates a Document
    pub fn new(label: &str) -> Self {
        Document {
            uuid: generate_id(),
            rev: Default::default(),
            data: Default::default(),
            seq: 0i64,
            label: label.to_owned(),
        }
    }

    /// Updates the fields of a Document
    pub fn update(mut self, updates: T) -> Self {
        self.data = updates;

        self
    }

    /// Saves Document to filesystem
    pub fn store(mut self) -> Result<Self, LocalLedgerError> {
        let data = serde_json::to_string(&self.data).map_err(|serde_err| {
            LocalLedgerError::new(&format!(
                "Failed to serialize document: {:?}",
                serde_err.to_string()
            ))
        })?;

        let mut h = DefaultHasher::new();

        data.hash(&mut h);

        let rev = h.finish().to_string();

        if rev != self.rev {
            self.seq += 1;
        }

        self.rev = rev;

        let doc_json = serde_json::to_string(&self).map_err(|serde_err| {
            LocalLedgerError::new(&format!(
                "Failed to serialize document: {}",
                serde_err.to_string()
            ))
        })?;

        let mut path = get_or_create_doc_dir(&self.label)?;

        path.push(format!("{}.json", self.uuid));

        let file_exists = path.exists();

        let mut doc_file = get_or_create_doc_file(&path)?;

        if file_exists {
            check_for_conflict::<T>(&mut doc_file, self.seq)?;
        }

        doc_file.write_all(doc_json.as_bytes()).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to save doc: {}", err.to_string()))
        })?;

        Ok(self)
    }

    /// Removes Document from filesystem
    pub fn remove(&mut self) -> Result<(), LocalLedgerError> {
        let mut path = get_or_create_doc_dir(&self.label)?;

        path.push(format!("{}.json", self.uuid));

        std::fs::remove_file(path).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to remove doc: {}", err.to_string()))
        })?;

        Ok(())
    }

    /// Loads Document from the filesystem
    pub fn load(self, uuid: &str) -> Result<Self, LocalLedgerError> {
        let contents = load_from_disc(uuid, &self.label)?;

        let doc: Self = serde_json::from_str(&contents).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to parse doc file: {}", err.to_string()))
        })?;

        Ok(doc)
    }

    /// Return read only Document data
    pub fn read_data<'a>(&'a self) -> &'a T {
        &self.data
    }

    /// Returns read only uuid
    pub fn read_uuid<'a>(&'a self) -> &'a str {
        &self.uuid
    }
}

fn load_from_disc(uuid: &str, label: &str) -> Result<String, LocalLedgerError> {
    let mut doc_file =
        std::fs::File::open(format!("./{}/{}.json", label, uuid)).map_err(|err| {
            LocalLedgerError::new(&format!("Document not found: {}", err.to_string()))
        })?;

    let mut contents = String::new();
    doc_file.read_to_string(&mut contents).unwrap();

    Ok(contents)
}

fn get_or_create_doc_dir(dir_name: &str) -> Result<PathBuf, LocalLedgerError> {
    let mut path = PathBuf::new();

    path.push(dir_name);

    std::fs::create_dir_all(&path).map_err(|err| {
        LocalLedgerError::new(&format!(
            "Failed to create document directory: {}",
            err.to_string()
        ))
    })?;

    Ok(path)
}

fn get_or_create_doc_file(file_path: &PathBuf) -> Result<std::fs::File, LocalLedgerError> {
    let doc_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(file_path)
        .map_err(|err| {
            LocalLedgerError::new(&format!("Failed to open file: {}", err.to_string()))
        })?;

    Ok(doc_file)
}

fn check_for_conflict<T: Clone + Serialize + DeserializeOwned + Default>(
    doc_file: &mut std::fs::File,
    new_seq: i64,
) -> Result<(), LocalLedgerError> {
    let mut curr_contents = String::new();

    doc_file.read_to_string(&mut curr_contents).map_err(|err| {
        LocalLedgerError::new(&format!("Failed to load file: {}", err.to_string()))
    })?;

    let curr_doc: Document<T> = serde_json::from_str(&curr_contents).map_err(|err| {
        LocalLedgerError::new(&format!(
            "Failed to parse previous doc file: {}",
            err.to_string()
        ))
    })?;

    let curr_seq = curr_doc.seq;

    let has_doc_update_conf = new_seq - curr_seq < DOCUMENT_CONFLICT_THRESHOLD;

    if has_doc_update_conf {
        return Err(LocalLedgerError::new("Document update conflict"));
    }

    doc_file.rewind().map_err(|err| {
        LocalLedgerError::new(&format!("Failed to reset file: {}", err.to_string()))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[derive(Serialize, Debug, Default, Clone, Deserialize, PartialEq, Eq)]
    struct Person {
        age: i32,
        name: String,
    }

    #[test]
    fn should_save_doc() {
        let person = Person {
            age: 1,
            name: "duder".to_owned(),
        };

        let doc: Document<Person> = Document::new("Person")
            .update(person.clone())
            .store()
            .unwrap();

        assert_eq!(person, doc.data);
    }

    #[test]
    fn should_load_doc() {
        let person = Person {
            age: 1,
            name: "duder".to_owned(),
        };

        let doc: Document<Person> = Document::new("Person")
            .update(person.clone())
            .store()
            .unwrap();

        let loaded_doc: Document<Person> = Document::new("Person").load(&doc.uuid).unwrap();

        let loaded_person = loaded_doc.read_data();

        assert_eq!(loaded_person, &person);
    }

    #[test]
    fn should_update_doc() {
        let person = Person {
            age: 1,
            name: "duder".to_owned(),
        };

        let mut doc: Document<Person> = Document::new("Person")
            .update(person.clone())
            .store()
            .unwrap();

        let updated_person = Person {
            age: 21,
            ..person.clone()
        };

        doc = doc.update(updated_person.clone()).store().unwrap();

        let loaded_doc: Document<Person> = Document::new("Person").load(doc.read_uuid()).unwrap();

        let loaded_person = loaded_doc.read_data();

        assert_eq!(loaded_person, &updated_person);
    }

    #[test]
    fn should_receive_document_update_conflict() {
        let person = Person {
            age: 21,
            name: "Duderino".to_owned(),
        };

        let mut doc_0 = Document::new("Person")
            .update(person.clone())
            .store()
            .unwrap();

        let doc_uuid = doc_0.read_uuid();

        let mut doc_1: Document<Person> = Document::new("Person").load(doc_uuid).unwrap();

        doc_0 = doc_0.update(Person {
            age: 31,
            ..person.clone()
        });
        doc_1 = doc_1.update(Person {
            age: 22,
            ..person.clone()
        });

        doc_0.store().unwrap();

        let err = doc_1.store().unwrap_err();

        assert_eq!(err.message, "Document update conflict".to_owned());
    }

    #[test]
    fn load_should_fail_if_doc_does_not_exist() {
        let maybe_doc: Result<Document<Person>, _> =
            Document::new("Person").load("some invalid uuid");

        let err = maybe_doc.unwrap_err();

        let contains_correct_msg = err.to_string().contains("Document not found: ");

        assert!(contains_correct_msg);
    }

    #[test]
    fn should_remove_doc() {
        let person = Person {
            age: 21,
            name: "Duderino".to_owned(),
        };

        let mut doc_0 = Document::new("Person")
            .update(person.clone())
            .store()
            .unwrap();

        let uuid = doc_0.read_uuid().to_owned();

        let _: Document<Person> = Document::new("Person").load(&uuid).unwrap();

        doc_0.remove().unwrap();

        let maybe_doc: Result<Document<Person>, _> = Document::new("Person").load(&uuid);

        let err = maybe_doc.unwrap_err();

        let contains_correct_msg = err.to_string().contains("Document not found: ");

        assert!(contains_correct_msg);
    }

    #[test]
    fn should_be_able_to_create_a_hash_map_document() {
        let mut hs = HashMap::new();
        hs.insert("hello".to_owned(), "world".to_owned());

        let hash_map_doc_0: Document<HashMap<String, String>> =
            Document::new("Config").update(hs.clone()).store().unwrap();

        let hash_map_doc_1: Document<HashMap<String, String>> = Document::new("Config")
            .load(hash_map_doc_0.read_uuid())
            .unwrap();

        let received_hash_map = hash_map_doc_1.read_data();

        assert_eq!(received_hash_map, &hs);
    }
}
