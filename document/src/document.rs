use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::Debug,
    io::{Read, Seek, Write},
    path::PathBuf,
};
use utility::{generate_id, LocalLedgerError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document<T> {
    label: String,
    uuid: String,
    rev: String,
    data: T,
    seq: i64,
    encrypted_data: Vec<u8>,
    encrypted: bool,
    has_been_decrypted: bool,
    rev_history: Vec<String>,
}

impl<T> Document<T>
where
    T: Clone + Serialize + DeserializeOwned + Default + Debug,
{
    /// Creates a Document with a specified label
    pub fn new(label: &str) -> Self {
        Document {
            uuid: generate_id(),
            rev: Default::default(),
            data: Default::default(),
            seq: 0i64,
            label: label.to_owned(),
            encrypted_data: Default::default(),
            encrypted: false,
            has_been_decrypted: false,
            rev_history: vec![],
        }
    }

    /// Creates a document with specified label and uuid
    pub fn new_alt(label: &str, uuid: &str) -> Self {
        // This name is kinda cringe.  I was torn here.  Thought of using an enum to get a pseudo
        // overloaded `new` method, but it felt like overkill.  I do not see anything other than
        // label and uuid needing to be provided to a constructor, but we will just have to wait
        // and see.
        Document {
            uuid: uuid.to_owned(),
            rev: Default::default(),
            data: Default::default(),
            seq: 0i64,
            label: label.to_owned(),
            encrypted_data: Default::default(),
            encrypted: false,
            has_been_decrypted: false,
            rev_history: vec![],
        }
    }

    pub fn remove_doc(label: &str, uuid: &str) -> Result<(), LocalLedgerError> {
        let mut path = get_or_create_doc_dir(label)?;
        path.push(format!("{}.json", uuid));

        std::fs::remove_file(path).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to remove doc: {}", err.to_string()))
        })?;

        Ok(())
    }

    /// Loads Document from the filesystem
    pub fn load(label: &str, uuid: &str) -> Result<Self, LocalLedgerError> {
        let contents = load_from_disc(uuid, &label)?;
        let doc = parse_doc::<T>(&contents)?;

        if doc.encrypted {
            return Err(LocalLedgerError::new("Load failed.  Data is encrypted"));
        }

        Ok(doc)
    }

    /// Tries to load document.  If it doesn't exist None is returned.
    pub fn try_load(label: &str, uuid: &str) -> Option<Self> {
        // If dir doesn't exist it is created.  Feels weird that a method
        // called try_load will sometimes make a write (creating a dir)
        // is this cringe?
        match get_or_create_doc_dir(label) {
            Ok(mut path) => {
                path.push(format!("{}.json", uuid));

                let file_exists = path.exists();

                if !file_exists {
                    return None;
                }

                match Document::<T>::load(label, uuid) {
                    Ok(doc) => Some(doc),
                    Err(_) => None,
                }
            }

            Err(_) => None,
        }
    }

    /// Loads Document from the filesystem, but calls decrypt transform function after reading from disk.
    pub fn decrypt_load<F>(label: &str, uuid: &str, decrypt: F) -> Result<Self, LocalLedgerError>
    where
        F: Fn(&Vec<u8>) -> Result<Vec<u8>, LocalLedgerError>,
    {
        let contents = load_from_disc(uuid, label)?;

        let mut parsed_doc = parse_doc(&contents)?;
        let decrypted_data = decrypt(&parsed_doc.encrypted_data)?;

        let parsed_data: T = serde_json::from_slice(&decrypted_data).map_err(|err| {
            LocalLedgerError::new(&format!(
                "Failed to parse decrypted data: {}",
                err.to_string()
            ))
        })?;

        parsed_doc.data = parsed_data;

        parsed_doc.has_been_decrypted = true;

        //let _ = std::mem::replace(self, parsed_doc);

        Ok(parsed_doc)
    }

    pub fn doc_exists(label: &str, uuid: &str) -> Result<bool, LocalLedgerError> {
        let mut path = get_dir_path(label)?;
        path.push(format!("{}.json", uuid));

        Ok(path.exists())
    }

    pub fn get_all_uuids(label: &str) -> Result<Vec<String>, LocalLedgerError> {
        let path = get_dir_path(label)?;
        let doc_uuids = std::fs::read_dir(path)
            .map_err(|e| LocalLedgerError::new(&e.to_string()))?
            .try_fold(
                Vec::<String>::new(),
                |mut accum, dir_entry_result| match dir_entry_result {
                    Ok(dir_entry) => {
                        if dir_entry.path().is_dir() {
                            return Ok(accum);
                        }

                        let path = dir_entry.path();

                        let file_name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .ok_or(LocalLedgerError::new("Failed to find document uuid"))?;

                        accum.push(file_name.to_string());

                        Ok(accum)
                    }

                    Err(e) => Err(LocalLedgerError::new(&format!(
                        "Failed to find ledger uuids: {}",
                        e.to_string()
                    ))),
                },
            )?;

        Ok(doc_uuids)
    }

    /// Updates the fields of a Document
    pub fn update<'a>(&'a mut self, updates: T) -> &'a mut Self {
        self.data = updates;

        self
    }

    /// Saves Document to filesystem
    pub fn store<'a>(&'a mut self) -> Result<&'a Self, LocalLedgerError> {
        if self.encrypted && !self.has_been_decrypted {
            return Err(LocalLedgerError::new(
                "Failed to store.  Document has been encrypted.  Try store_encrypted instead.",
            ));
        }

        self.do_store()
    }

    /// Saves Document to filesystem, but calls encrypt transform function before writing to disk.
    ///
    /// If successfull, this method clears the data currently being held in the Document.  Calling `read_data` afterward will give you default values and will not match what was saved to disk.  You must call `decrypt_load` in order to get the data again.
    pub fn store_encrypted<'a, F>(&'a mut self, encrypt: F) -> Result<&'a Self, LocalLedgerError>
    where
        F: Fn(Vec<u8>) -> Result<Vec<u8>, LocalLedgerError>,
    {
        let data = self.stringify_data()?;
        let encrypted_data = encrypt(data.into_bytes())?;

        self.encrypted_data = encrypted_data;
        self.encrypted = true;

        // If we make the has_been_decrypted field public we need to write a test for it
        self.has_been_decrypted = false;
        self.do_store()
    }

    /// Removes Document from filesystem
    pub fn remove(&mut self) -> Result<(), LocalLedgerError> {
        Document::<T>::remove_doc(&self.label, &self.uuid)
    }

    /// Return read only Document data
    pub fn read_data<'a>(&'a self) -> Result<&'a T, LocalLedgerError> {
        if !self.encrypted {
            return Ok(&self.data);
        }

        if self.encrypted && self.has_been_decrypted {
            return Ok(&self.data);
        }

        Err(LocalLedgerError::new(
            "Document is encrypted.  Please use decrypt_load in order to read this document",
        ))
    }

    pub fn read_mut<'a>(&'a mut self) -> Result<&'a mut T, LocalLedgerError> {
        if !self.encrypted {
            return Ok(&mut self.data);
        }

        if self.encrypted && self.has_been_decrypted {
            return Ok(&mut self.data);
        }

        Err(LocalLedgerError::new(
            "Document is encrypted.  Please use decrypt_load in order to read this document",
        ))
    }

    /// Returns read only uuid
    pub fn read_uuid<'a>(&'a self) -> &'a str {
        &self.uuid
    }

    pub fn append_uuid(&mut self, uuid: &str) {
        self.uuid = uuid.to_owned();
    }

    /// Returns an ownable copy of the document uuid
    pub fn get_uuid(&self) -> String {
        self.uuid.clone()
    }

    pub fn has_been_decrypted(&self) -> bool {
        self.has_been_decrypted
    }

    pub fn get_data_dir(&self) -> Result<PathBuf, LocalLedgerError> {
        get_dir_path(&self.label)
    }

    pub fn rev(&self) -> &str {
        &self.rev
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    fn do_store<'a>(&'a mut self) -> Result<&'a Self, LocalLedgerError> {
        let new_rev = generate_id();

        self.seq += 1;

        if self.seq > 1 {
            let prev_rev = self.rev.clone();
            self.rev_history.push(prev_rev);
        }

        self.rev = new_rev;

        let mut doc_json =
            serde_json::to_value(&self).map_err(|e| LocalLedgerError::new(&e.to_string()))?;

        if self.encrypted {
            // we dont want to write the data field to disc
            // This is probably a better way to do this... YOLO!
            let data_json = serde_json::to_value(T::default())
                .map_err(|e| LocalLedgerError::new(&e.to_string()))?;

            doc_json["data"] = data_json;
        }

        let doc_json_str = serde_json::to_string(&doc_json).map_err(|serde_err| {
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
            check_for_conflict::<T>(&mut doc_file, &self.rev_history, &self.rev)?;
        }

        let doc_bytes = doc_json_str.as_bytes();

        // Set length of file to insure we are replacing the contents.
        // This can be done via call the .truncate() in get_or_create_doc_file
        doc_file.set_len(doc_bytes.len() as u64).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to set file length: {}", err.to_string()))
        })?;

        doc_file.write_all(doc_bytes).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to save doc: {}", err.to_string()))
        })?;

        Ok(self)
    }

    fn stringify_data(&self) -> Result<String, LocalLedgerError> {
        let data = serde_json::to_string(&self.data).map_err(|serde_err| {
            LocalLedgerError::new(&format!(
                "Failed to serialize document: {:?}",
                serde_err.to_string()
            ))
        })?;

        Ok(data)
    }
}

fn load_from_disc(uuid: &str, label: &str) -> Result<String, LocalLedgerError> {
    //let path = format!("{}/{}.json", get_dir_path(label), uuid);
    let mut path = get_dir_path(label)?;

    path.push(format!("{}.json", uuid));

    let mut doc_file = std::fs::File::open(path).map_err(|err| {
        LocalLedgerError::new(&format!("Document not found: {}", err.to_string()))
    })?;

    let mut contents = String::new();
    doc_file.read_to_string(&mut contents).unwrap();

    Ok(contents)
}

fn get_or_create_doc_dir(doc_label: &str) -> Result<PathBuf, LocalLedgerError> {
    let path = get_dir_path(doc_label)?;

    std::fs::create_dir_all(&path).map_err(|err| {
        LocalLedgerError::new(&format!(
            "Failed to create document directory: {}",
            err.to_string()
        ))
    })?;

    Ok(path)
}

fn get_dir_path(doc_label: &str) -> Result<PathBuf, LocalLedgerError> {
    let mut base_dir = dirs::home_dir().map_or(
        Err(LocalLedgerError::new("Failed to get directory path")),
        |d| Ok(d),
    )?;

    base_dir.push(".fizzgig");
    base_dir.push(doc_label);

    Ok(base_dir)
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

fn parse_doc<T: Clone + Serialize + DeserializeOwned + Default + Debug>(
    contents: &str,
) -> Result<Document<T>, LocalLedgerError> {
    let doc: Document<T> = serde_json::from_str(&contents).map_err(|err| {
        LocalLedgerError::new(&format!("Failed to parse doc file: {}", err.to_string()))
    })?;

    Ok(doc)
}

fn check_for_conflict<T: Clone + Serialize + DeserializeOwned + Default + Debug>(
    doc_file: &mut std::fs::File,
    new_rev_history: &Vec<String>,
    new_rev: &str,
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

    let new_rev_history_iter = new_rev_history.into_iter();
    let curr_rev_history_iter = curr_doc.rev_history.into_iter();

    if new_rev_history_iter.len() >= curr_rev_history_iter.len() {
        let new_rev = new_rev.to_string();
        let mut side_by_side_history = curr_rev_history_iter
            .chain(vec![curr_doc.rev])
            .zip(new_rev_history_iter.chain(vec![&new_rev]));

        while let Some((rev_1, rev_2)) = side_by_side_history.next() {
            if &rev_1 != rev_2 {
                return Err(LocalLedgerError::conflict("Document update conflict"));
            }
        }
    } else {
        return Err(LocalLedgerError::conflict("Document update conflict"));
    }

    doc_file.rewind().map_err(|err| {
        LocalLedgerError::new(&format!("Failed to reset file: {}", err.to_string()))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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

        let mut doc = Document::new("Person");

        doc.update(person.clone()).store().unwrap();

        assert_eq!(person, doc.data);

        doc.remove().unwrap();
    }

    #[test]
    fn should_load_doc() {
        let person = Person {
            age: 1,
            name: "duder".to_owned(),
        };

        let mut doc = Document::new("Person");
        doc.update(person.clone()).store().unwrap();

        let mut loaded_doc = Document::<Person>::load("Person", &doc.uuid).unwrap();
        let loaded_person = loaded_doc.read_data().unwrap();

        assert_eq!(loaded_person, &person);

        loaded_doc.remove().unwrap();
    }

    #[test]
    fn should_update_doc() {
        let person = Person {
            age: 1,
            name: "duder".to_owned(),
        };

        let mut doc = Document::new("Person");
        doc.update(person.clone()).store().unwrap();

        let updated_person = Person {
            age: 21,
            ..person.clone()
        };

        doc.update(updated_person.clone()).store().unwrap();

        let loaded_doc = Document::<Person>::load("Person", &doc.uuid).unwrap();
        let loaded_person = loaded_doc.read_data().unwrap();

        assert_eq!(loaded_person, &updated_person);

        doc.remove().unwrap();
    }

    #[test]
    fn should_receive_document_update_conflict() {
        let person = Person {
            age: 21,
            name: "Duderino".to_owned(),
        };

        let mut doc_0 = Document::new("Person");
        doc_0.update(person.clone()).store().unwrap();

        let doc_uuid = doc_0.read_uuid();

        let mut doc_1 = Document::<Person>::load("Person", doc_uuid).unwrap();

        doc_0.update(Person {
            age: 31,
            ..person.clone()
        });
        doc_1.update(Person {
            age: 22,
            ..person.clone()
        });

        doc_0.store().unwrap();

        let err = doc_1.store().unwrap_err();

        assert_eq!(err.message, "Document update conflict".to_owned());

        doc_0.remove().unwrap();
    }

    #[test]
    fn load_should_fail_if_doc_does_not_exist() {
        let err = Document::<Person>::load("Person", "some invalid id").unwrap_err();

        let contains_correct_msg = err.to_string().contains("Document not found: ");

        assert!(contains_correct_msg);
    }

    #[test]
    fn should_remove_doc() {
        let person = Person {
            age: 21,
            name: "Duderino".to_owned(),
        };

        let mut doc_0 = Document::new("Person");
        doc_0.update(person.clone()).store().unwrap();

        let uuid = doc_0.read_uuid().to_owned();

        doc_0.remove().unwrap();

        let err = Document::<Person>::load("Person", &uuid).unwrap_err();

        let contains_correct_msg = err.to_string().contains("Document not found: ");

        assert!(contains_correct_msg);
    }

    #[test]
    fn should_be_able_to_create_a_hash_map_document() {
        let mut hs = HashMap::new();
        hs.insert("hello".to_owned(), "world".to_owned());

        let mut hash_map_doc_0 = Document::<HashMap<String, String>>::new("Config");
        hash_map_doc_0.update(hs.clone()).store().unwrap();

        let hash_map_doc_1 =
            Document::<HashMap<String, String>>::load("Config", hash_map_doc_0.read_uuid())
                .unwrap();
        //hash_map_doc_1.load(hash_map_doc_0.read_uuid()).unwrap();

        let received_hash_map = hash_map_doc_1.read_data().unwrap();

        assert_eq!(received_hash_map, &hs);

        hash_map_doc_0.remove().unwrap();
    }

    #[test]
    fn should_store_encrypted_data() {
        let person = Person {
            age: 21,
            name: "Duderino".to_owned(),
        };

        let mut doc_0 = Document::new("Person");

        doc_0
            .update(person.clone())
            .store_encrypted(|_data| Ok(b"ENCRYPTED_DATA".to_vec()))
            .unwrap();

        let doc_1 = Document::<Person>::decrypt_load("Person", &doc_0.uuid, |_encrypted_data| {
            let decrypted_data = serde_json::to_vec(&person).unwrap();

            Ok(decrypted_data)
        })
        .unwrap();

        assert_eq!(doc_0.read_uuid(), doc_1.read_uuid());

        assert_eq!(doc_1.read_data().unwrap(), &person);

        doc_0.remove().unwrap();
    }

    #[test]
    fn load_should_fail_if_doc_is_encrypted() {
        let person = Person {
            age: 21,
            name: "Duderino".to_owned(),
        };

        let mut doc_0 = Document::new("Person");

        doc_0
            .update(person.clone())
            .store_encrypted(|_data| Ok(b"ENCRYPTED_DATA".to_vec()))
            .unwrap();

        let failed_doc = Document::<Person>::load("Person", &doc_0.uuid).unwrap_err();

        let received_err_msg = &failed_doc.message;

        let expected_err_msg = "Load failed.  Data is encrypted";

        assert_eq!(received_err_msg, expected_err_msg);

        doc_0.remove().unwrap();
    }

    #[test]
    fn store_should_fail_if_doc_is_encrypted() {
        let person = Person {
            age: 21,
            name: "Duderino".to_owned(),
        };

        let mut doc_0 = Document::new("Person");

        doc_0
            .update(person.clone())
            .store_encrypted(|_data| Ok(b"ENCRYPTED_DATA".to_vec()))
            .unwrap();

        let person_update = Person { age: 18, ..person };

        let failed_update = doc_0.update(person_update).store().unwrap_err();

        let received_err_msg = &failed_update.message;

        let expected_err_msg =
            "Failed to store.  Document has been encrypted.  Try store_encrypted instead.";

        assert_eq!(received_err_msg, expected_err_msg);

        //remove_doc("Person", uuid)
        doc_0.remove().unwrap();
    }

    #[test]
    fn should_be_able_to_query_document_for_its_decrypted_state() {
        let person = Person {
            age: 21,
            name: "Duderino".to_owned(),
        };

        let mut doc_0 = Document::new("Person");

        doc_0
            .update(person.clone())
            .store_encrypted(|_data| Ok(b"ENCRYPTED_DATA".to_vec()))
            .unwrap();

        assert!(!doc_0.has_been_decrypted());

        let mut doc_1 = Document::<Person>::decrypt_load(&doc_0.label, &doc_0.uuid, |_| {
            let decrypted_data = serde_json::to_vec(&person).unwrap();

            Ok(decrypted_data)
        })
        .unwrap();

        assert!(doc_1.has_been_decrypted());

        doc_1
            .update(Person {
                name: "Duder".to_owned(),
                ..person
            })
            .store_encrypted(|_data| Ok(b"UPDATED_ENCRYPTED_DATA".to_vec()))
            .unwrap();

        assert!(!doc_0.has_been_decrypted());

        doc_0.remove().unwrap();
    }

    #[test]
    fn should_be_able_to_store_doc_with_no_changes() {
        let person = Person {
            age: 21,
            name: "duder".to_owned(),
        };

        let mut doc_0 = Document::new("Person");

        doc_0.update(person.clone()).store().unwrap();

        doc_0
            .update(person.clone())
            .store()
            .expect("Failed to store");

        doc_0.remove().unwrap();
    }

    #[test]
    fn should_not_overwrite_with_multiple_calls_to_store_encrypted() {
        let person = Person {
            age: 21,
            name: "duder".to_string(),
        };
        let mut doc_0 = Document::new("Person");

        doc_0
            .update(person.clone())
            .store_encrypted(|d| Ok(d))
            .expect("Failed to store");

        doc_0.store_encrypted(|d| Ok(d)).expect("Failed to store");

        assert_eq!(doc_0.data.name, "duder".to_string());
        assert_eq!(doc_0.data.age, 21);

        doc_0.remove().unwrap();
    }

    // #[test]
    // fn should_alksdfjasdlkfj() {
    //     let person = Person {
    //         age: 21,
    //         name: "duder".to_string(),
    //     };
    //     let mut doc_0 = Document::new("Person");
    //     doc_0
    //         .update(person.clone())
    //         .store_encrypted(|d| Ok(d))
    //         .expect("Failed to store");

    //     let mut loaded_doc =
    //         Document::<Person>::decrypt_load(&doc_0.label, &doc_0.uuid, |i| Ok(i.clone())).unwrap();
    // }
}
