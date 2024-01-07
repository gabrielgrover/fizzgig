use crate::LedgerDump;
use age::secrecy::Secret;
use document::Document;
use pwhash::bcrypt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Read, Write},
    num::NonZeroUsize,
    path::PathBuf,
    pin::Pin,
};
use tokio_stream::{Stream, StreamExt};
use utility::{LocalLedgerError, LocalLedgerErrorType};

const CONFLICT_SUFFIX: &str = "_conflicts";

#[derive(Debug)]
pub struct LocalLedger<T> {
    pub name: String,
    doc_cache: lru::LruCache<String, Document<T>>,
    assoc_doc: Document<HashMap<String, String>>,
    meta_doc: Document<LocalLedgerMetaData>,
    /// This field functions as an assoc doc for conflicts
    merge_conflict_doc: Document<HashMap<String, String>>,
    conflict_dir: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct LocalLedgerMetaData {
    pw_hash: String,
}

impl<T> LocalLedger<T>
where
    T: Clone + Serialize + DeserializeOwned + Default + Debug,
{
    pub fn new(name: &str, ledger_password: String) -> Result<Self, LocalLedgerError> {
        let cache_size = match NonZeroUsize::new(100) {
            Some(size) => Ok(size),
            None => Err(LocalLedgerError::new("Failed to initialize doc cache")),
        }?;
        let doc_cache = lru::LruCache::new(cache_size);
        let assoc_doc = create_assoc_doc(name);
        let conflict_dir = format!("{}_{}", name, CONFLICT_SUFFIX);
        let merge_conflict_doc = create_conflict_doc(&conflict_dir);
        let maybe_meta_doc = try_load_meta_doc(name);

        let meta_doc = match maybe_meta_doc {
            Some(loaded_meta_doc) => {
                let loaded_pw_hash = loaded_meta_doc.read_data()?.pw_hash.as_str();
                let correct_pw = bcrypt::verify(ledger_password.as_str(), loaded_pw_hash);

                if !correct_pw {
                    return Err(LocalLedgerError::new("Incorrect password"));
                }

                loaded_meta_doc
            }
            None => {
                let pw_hash = bcrypt::hash(&ledger_password).map_err(|err| {
                    LocalLedgerError::new(&format!(
                        "LocalLedger creation failed: {}",
                        err.to_string()
                    ))
                })?;
                let mut created_doc = create_meta_doc(name);

                created_doc.update(LocalLedgerMetaData { pw_hash });
                created_doc.store()?;
                created_doc
            }
        };

        Ok(LocalLedger {
            name: name.to_owned(),
            doc_cache,
            assoc_doc,
            meta_doc,
            merge_conflict_doc,
            conflict_dir,
        })
    }

    /// Creates a new entry to the ledger.  Returning a uuid.
    pub fn create(&mut self, data: T, label: &str) -> Result<String, LocalLedgerError> {
        if label.len() == 0 {
            return Err(LocalLedgerError::new("Label cannot be empty"));
        }

        let label_already_in_use = self.assoc_doc.read_data()?.contains_key(label);

        if label_already_in_use {
            return Err(LocalLedgerError::new("Labels must be unique"));
        }

        let mut encrypted_doc = Document::<T>::new(&self.name);

        encrypted_doc.update(data);
        encrypt_store_doc(&mut encrypted_doc, &self.meta_doc.read_data()?.pw_hash)?;

        let doc_uuid = encrypted_doc.get_uuid();

        self.doc_cache.put(doc_uuid.clone(), encrypted_doc);

        let label_doc_uuid_map = self.assoc_doc.read_mut()?;
        let _ = label_doc_uuid_map.insert(label.to_owned(), doc_uuid.clone());

        self.assoc_doc.store()?;

        Ok(doc_uuid)
    }

    /// Reads data in a document
    pub fn read<'a>(&'a mut self, uuid: String) -> Result<&'a T, LocalLedgerError> {
        let doc_is_cached = self.doc_cache.contains(&uuid);
        let key = &self.meta_doc.read_data()?.pw_hash;

        if doc_is_cached {
            let mut cached_doc = self.doc_cache.get_mut(&uuid).map_or(
                Err(LocalLedgerError::new("Failed to get doc from cache")),
                |d| Ok(d),
            )?;

            if !cached_doc.has_been_decrypted() {
                decrypt_load_doc(&mut cached_doc, &uuid, &key)?;
            }

            return cached_doc.read_data();
        }

        let mut loaded_doc = Document::<T>::new(&self.name);

        decrypt_load_doc(&mut loaded_doc, &uuid, &key)?;

        self.doc_cache.put(uuid.clone(), loaded_doc);

        let cached_doc = self.doc_cache.get(&uuid).map_or(
            Err(LocalLedgerError::new("Failed to get doc from cache")),
            |d| Ok(d),
        )?;

        cached_doc.read_data()
    }

    pub fn read_by_entry_name<'a>(
        &'a mut self,
        entry_name: &str,
    ) -> Result<&'a T, LocalLedgerError> {
        let uuid = self.assoc_doc.read_data()?.get(entry_name).map_or(
            Err(LocalLedgerError::new(&format!(
                "Ledger entry with name: {} does not exist",
                entry_name
            ))),
            |uuid| Ok(uuid),
        )?;

        self.read(uuid.to_string())
    }

    /// Updates document for given `label` with given `data`
    pub fn update(&mut self, label: &str, data: T) -> Result<(), LocalLedgerError> {
        let uuid = self
            .assoc_doc
            .read_data()?
            .get(label)
            .map_or(Err(LocalLedgerError::new("Label not found")), |i| Ok(i))?;
        let doc_is_cached = self.doc_cache.contains(uuid);
        let key = &self.meta_doc.read_data()?.pw_hash;

        if doc_is_cached {
            let mut cached_doc = self.doc_cache.get_mut(uuid).map_or(
                Err(LocalLedgerError::new("Failed to get doc from cached")),
                |d| Ok(d),
            )?;

            if !cached_doc.has_been_decrypted() {
                decrypt_load_doc(cached_doc, uuid, key)?;
            }

            cached_doc.update(data);

            encrypt_store_doc(&mut cached_doc, &self.meta_doc.read_data()?.pw_hash)?;

            return Ok(());
        }

        let mut doc = Document::<T>::new(&self.name);

        decrypt_load_doc(&mut doc, uuid, &self.meta_doc.read_data()?.pw_hash)?;

        doc.update(data);

        encrypt_store_doc(&mut doc, &self.meta_doc.read_data()?.pw_hash)?;

        self.doc_cache.put(uuid.to_owned(), doc);

        Ok(())
    }

    pub fn remove(&mut self, entry_name: &str) -> Result<(), LocalLedgerError> {
        let uuid = self
            .assoc_doc
            .read_data()?
            .get(entry_name)
            .ok_or(LocalLedgerError::new(&format!(
                "Ledger entry with name: {} does not exist",
                entry_name
            )))?
            .as_str();
        let doc_is_cached = self.doc_cache.contains(uuid);

        if doc_is_cached {
            let cached_doc = self
                .doc_cache
                .get_mut(uuid)
                .ok_or(LocalLedgerError::new("Failed to get doc from cache"))?;

            cached_doc.remove()?;

            let _ = self.doc_cache.pop_entry(uuid);
        } else {
            Document::<T>::remove_doc(&self.name, uuid)?;
        }

        let assoc_doc = self.assoc_doc.read_mut()?;
        let _ = assoc_doc.remove(entry_name);
        let _ = self.assoc_doc.store()?;

        Ok(())
    }

    pub fn list_entry_labels<'a>(&'a self) -> Result<Vec<&'a str>, LocalLedgerError> {
        let assoc_doc = self.assoc_doc.read_data()?;
        let mut labels = Vec::new();

        for key in assoc_doc.keys() {
            labels.push(key.as_str());
        }

        Ok(labels)
    }

    pub fn get_ledger_dir(&self) -> Result<PathBuf, LocalLedgerError> {
        self.assoc_doc.get_data_dir()
    }

    /// Retrieves all ledger's contents into a Read implementation.  Each doc is separated by a `\n` char
    pub fn doc_dump(&self) -> Result<LedgerDump, LocalLedgerError> {
        let src_dir = self.get_ledger_dir()?;
        let ld = LedgerDump::new(src_dir).map_err(|msg| LocalLedgerError::new(&msg))?;

        Ok(ld)
    }

    pub async fn merge<S>(&mut self, mut s: S) -> Result<(), String>
    where
        S: Stream<Item = Result<Value, Box<dyn std::error::Error>>> + Unpin,
    {
        while let Some(item) = s.next().await {
            let val = item.map_err(|e| e.to_string())?;
            let uuid = assert_str(&val["uuid"])?;

            let store_result = match uuid.as_str() {
                "ASSOC_DOC" => {
                    let mut incomming_assoc_doc =
                        serde_json::from_value::<Document<HashMap<String, String>>>(val)
                            .map_err(|e| e.to_string())?;

                    incomming_assoc_doc.store().map(|_| ()) //.map_err(|e| e.to_string())?;
                }

                "META_DOC" => {
                    let mut incomming_meta_doc =
                        serde_json::from_value::<Document<LocalLedgerMetaData>>(val)
                            .map_err(|e| e.to_string())?;

                    incomming_meta_doc.store().map(|_| ()) //map_err(|e| e.to_string())?;
                }

                _ => {
                    let mut incomming_ledger_doc =
                        serde_json::from_value::<Document<T>>(val).map_err(|e| e.to_string())?;

                    encrypt_store_doc(
                        &mut incomming_ledger_doc,
                        &self
                            .meta_doc
                            .read_data()
                            .map_err(|e| e.to_string())?
                            .pw_hash,
                    )
                    .map(|_| ())
                    //.map_err(|e| e.to_string())?;
                }
            };

            if store_result.is_err() {
                let err = store_result.unwrap_err();

                match err.err_type {
                    LocalLedgerErrorType::Confict => {
                        //handle conflict

                        return Err(err.to_string());
                    }

                    LocalLedgerErrorType::Default => {
                        return Err(err.to_string());
                    }
                }
            }
        }

        Ok(())
    }
}

fn decrypt_load_doc<T: Clone + Serialize + DeserializeOwned + Default + Debug>(
    curr_doc: &mut Document<T>,
    uuid: &str,
    key: &str,
) -> Result<(), LocalLedgerError> {
    let loaded_doc = Document::<T>::decrypt_load(curr_doc.label(), &uuid, |encrypted_data| {
        let decryptor = match age::Decryptor::new(&encrypted_data[..]).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string()))
        })? {
            age::Decryptor::Passphrase(d) => Ok(d),
            _ => Err(LocalLedgerError::new("Failed to decrypt. Received encrypted data that was secured by some means other than a passphrase."))
        }?;

        let mut decrypted = vec![];
        let mut reader = decryptor
            .decrypt(&Secret::new(key.to_owned()), None)
            .map_err(|err| {
                LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string()))
            })?;

        reader.read_to_end(&mut decrypted).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string()))
        })?;

        Ok(decrypted)
    })?;

    let _ = std::mem::replace(curr_doc, loaded_doc);

    Ok(())
}

fn encrypt_store_doc<T: Clone + Serialize + DeserializeOwned + Default + Debug>(
    doc: &mut Document<T>,
    key: &str,
) -> Result<(), LocalLedgerError> {
    doc.store_encrypted(|data| {
        let encryptor = age::Encryptor::with_user_passphrase(Secret::new(key.to_owned()));
        let mut encrypted_data = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted_data).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to encrypt doc: {}", err.to_string()))
        })?;

        writer.write_all(&data).map_err(|err| {
            LocalLedgerError::new(&format!("Failed to encrypt doc: {}", err.to_string()))
        })?;

        writer.finish().map_err(|err| {
            LocalLedgerError::new(&format!("Failed to encrypt doc: {}", err.to_string()))
        })?;

        Ok(encrypted_data)
    })?;

    Ok(())
}

/// Attempts to create an assoc doc.  If it already exists it is loaded into memory.  
fn create_assoc_doc(ledger_name: &str) -> Document<HashMap<String, String>> {
    Document::<HashMap<String, String>>::try_load(ledger_name, "ASSOC_DOC").unwrap_or_else(|| {
        let mut assoc_doc = Document::new(ledger_name);

        assoc_doc.append_uuid("ASSOC_DOC");

        assoc_doc
    })
}

/// Attempts to create a conflict doc.  If it already exists it is loaded into memory.  
fn create_conflict_doc(name: &str) -> Document<HashMap<String, String>> {
    Document::<HashMap<String, String>>::try_load(name, "CONFLICT_DOC")
        .unwrap_or(Document::new(name))
}

fn try_load_meta_doc(ledger_name: &str) -> Option<Document<LocalLedgerMetaData>> {
    match Document::<LocalLedgerMetaData>::load(ledger_name, "META_DOC") {
        Ok(meta_doc) => Some(meta_doc),
        Err(_err) => None,
    }
}

fn create_meta_doc(ledger_name: &str) -> Document<LocalLedgerMetaData> {
    let mut meta_doc = Document::<LocalLedgerMetaData>::new(ledger_name);

    meta_doc.append_uuid("META_DOC");

    meta_doc
}

fn assert_str(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.to_owned()),

        _ => Err("Value is not a string".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
    struct Person {
        age: i32,
        name: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
    struct SavedPassword {
        pw: String,
        name: String,
    }

    #[test]
    #[serial]
    fn should_check_pw_for_existing_ledgers() {
        let _initial_ledger = LocalLedger::<Person>::new("Users", "password".to_owned()).unwrap();
        let open_initial_ledger_attemp =
            LocalLedger::<Person>::new("Users", "wrong password".to_owned());

        let err = open_initial_ledger_attemp.unwrap_err();
        let expected_msg = "Incorrect password".to_string();

        assert_eq!(err.to_string(), expected_msg);
    }

    #[test]
    #[serial]
    fn should_be_able_to_write_to_the_ledger() {
        let person = Person {
            age: 21,
            name: "duderino".to_owned(),
        };

        let mut user_ledger = LocalLedger::<Person>::new("Users", "password".to_owned()).unwrap();

        user_ledger
            .create(person.clone(), "employee-1")
            .expect("Failed to write to ledger");

        user_ledger.remove("employee-1").unwrap();
    }

    #[test]
    #[serial]
    fn should_be_able_to_read_ledger() {
        let person = Person {
            age: 21,
            name: "duderino".to_owned(),
        };

        let mut user_ledger = LocalLedger::<Person>::new("Users", "password".to_owned()).unwrap();

        let uuid = user_ledger
            .create(person.clone(), "employee-2")
            .expect("Failed to write to ledger");

        let received_data = user_ledger.read(uuid).unwrap();

        assert_eq!(received_data, &person);

        user_ledger.remove("employee-2").unwrap();
    }

    #[test]
    #[serial]
    fn should_be_able_to_read_in_sequence() {
        let person = Person {
            age: 21,
            name: "duderino".to_owned(),
        };

        let person_1 = Person {
            name: "walter".to_owned(),
            ..person
        };

        let mut user_ledger = LocalLedger::<Person>::new("Users", "password".to_owned()).unwrap();

        let uuid_0 = user_ledger
            .create(person.clone(), "employee-3")
            .expect("Failed to write to ledger");

        let received_data_0 = user_ledger.read(uuid_0).unwrap();

        assert_eq!(received_data_0, &person);

        let uuid_1 = user_ledger
            .create(person_1.clone(), "employee-4")
            .expect("Failed to write to ledger");

        let received_data_1 = user_ledger.read(uuid_1).unwrap();

        assert_eq!(received_data_1, &person_1);

        user_ledger.remove("employee-3").unwrap();
        user_ledger.remove("employee-4").unwrap();
    }

    #[test]
    #[serial]
    fn should_update_doc() {
        let person = Person {
            age: 21,
            name: "duderino".to_owned(),
        };

        let mut user_ledger = LocalLedger::<Person>::new("Users", "password".to_owned()).unwrap();

        let uuid = user_ledger
            .create(person.clone(), "employee-5")
            .expect("Failed to write to ledger");

        user_ledger
            .update(
                "employee-5",
                Person {
                    age: 25,
                    ..person.clone()
                },
            )
            .expect("Failed to update ledger");

        let received_data = user_ledger.read(uuid).unwrap();

        let expected_data = Person { age: 25, ..person };

        assert_eq!(received_data, &expected_data);

        user_ledger.remove("employee-5").unwrap();
    }

    #[test]
    #[serial]
    fn should_retrieve_all_doc_labels() {
        let s_pw_1 = SavedPassword {
            name: "www.example.com".to_owned(),
            pw: "password1234".to_owned(),
        };

        let s_pw_2 = SavedPassword {
            name: "www.helloworld.com".to_owned(),
            pw: "abc123".to_owned(),
        };

        let mut user_ledger =
            LocalLedger::<SavedPassword>::new("Passwords", "master_password".to_owned()).unwrap();

        user_ledger
            .create(s_pw_1.clone(), "my example.com password")
            .expect("Failed to write to ledger.");

        user_ledger
            .create(s_pw_2.clone(), "my helloworld.com password")
            .expect("Failed to write to ledger.");

        let received_data = user_ledger.list_entry_labels().unwrap();

        let mut found_entries = 0;

        for label in received_data.into_iter() {
            if label == "my example.com password" {
                found_entries += 1;
            }

            if label == "my helloworld.com password" {
                found_entries += 1;
            }
        }

        assert_eq!(found_entries, 2);

        user_ledger.remove("my example.com password").unwrap();

        user_ledger.remove("my helloworld.com password").unwrap();
    }

    #[test]
    #[serial]
    fn should_return_err_if_label_is_blank() {
        let s_pw_1 = SavedPassword {
            name: "www.example.com".to_owned(),
            pw: "password1234".to_owned(),
        };

        let mut user_ledger =
            LocalLedger::<SavedPassword>::new("Passwords", "master_password".to_owned()).unwrap();

        let err = user_ledger.create(s_pw_1, "").unwrap_err();

        assert_eq!(err, LocalLedgerError::new("Label cannot be empty"));
    }

    #[test]
    #[serial]
    fn should_return_err_if_label_is_not_unique() {
        let s_pw_1 = SavedPassword {
            name: "www.example.com".to_owned(),
            pw: "password1234".to_owned(),
        };

        let s_pw_2 = SavedPassword {
            name: "www.helloworld.com".to_owned(),
            pw: "abc123".to_owned(),
        };

        let mut user_ledger =
            LocalLedger::<SavedPassword>::new("Passwords", "master_password".to_owned()).unwrap();

        user_ledger
            .create(s_pw_1.clone(), "my password")
            .expect("Failed to write to ledger.");

        let err = user_ledger
            .create(s_pw_2.clone(), "my password")
            .unwrap_err();

        assert_eq!(err, LocalLedgerError::new("Labels must be unique"));

        user_ledger.remove("my password").unwrap();
    }
}
