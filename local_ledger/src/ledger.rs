use crate::LedgerDump;
use age::secrecy::{ExposeSecret, Secret};
use document::Document;
use pwhash::bcrypt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{
    fmt::Debug,
    io::{Read, Write},
    num::NonZeroUsize,
    path::PathBuf,
};
use tokio_stream::{Stream, StreamExt};
use utility::LocalLedgerError;

const META_DOC_UUID: &str = "META_DOC";

#[derive(Debug)]
pub struct LocalLedger<T> {
    // This is the name of the ledger, but also functions as the label that is used for the
    // Documents managed by the ledger
    pub name: String,
    doc_cache: lru::LruCache<String, Document<T>>,
    meta_doc: Document<LocalLedgerMetaData>,
    pw: Secret<String>,
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
            //assoc_doc,
            meta_doc,
            pw: Secret::new(ledger_password),
        })
    }

    /// Creates a new entry to the ledger.  Returning a uuid.
    pub fn create(&mut self, data: T, entry_name: &str) -> Result<String, LocalLedgerError> {
        if entry_name.len() == 0 {
            return Err(LocalLedgerError::new("Label cannot be empty"));
        }

        let label_already_in_use = self.entry_name_already_in_use(entry_name)?;

        if label_already_in_use {
            return Err(LocalLedgerError::new("Labels must be unique"));
        }

        let mut encrypted_doc = Document::<T>::new(&self.name);
        encrypted_doc.append_uuid(entry_name);
        encrypted_doc.update(data);
        encrypt_store_doc(&mut encrypted_doc, &self.pw.expose_secret())?;

        let doc_uuid = encrypted_doc.get_uuid();

        self.doc_cache.put(doc_uuid.clone(), encrypted_doc);

        Ok(doc_uuid)
    }

    /// Reads data in a document
    pub fn read<'a>(&'a mut self, uuid: String) -> Result<&'a T, LocalLedgerError> {
        let doc_is_cached = self.doc_cache.contains(&uuid);
        let key = &self.pw.expose_secret();

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

        tracing::info!("calling decrypt load");
        decrypt_load_doc(&mut loaded_doc, &uuid, &key)?;
        tracing::info!("decrypt load success");

        self.doc_cache.put(uuid.clone(), loaded_doc);

        let cached_doc = self
            .doc_cache
            .get(&uuid)
            .ok_or(LocalLedgerError::new("Failed to get doc from cache"))?;

        cached_doc.read_data()
    }

    pub fn get_conf(&mut self, entry_name: &str) -> Result<Document<T>, LocalLedgerError> {
        let key = &self.pw.expose_secret();
        let conf_doc = decrypt_load_conf::<T>(&self.name, key, entry_name)?;

        Ok(conf_doc)
    }

    pub fn resolve(
        &mut self,
        entry_name: &str,
        keep_original: bool,
    ) -> Result<(), LocalLedgerError> {
        // There is knowledge of the internals of the Document struct embedded in this logic.
        // This methods knows that the uuids of the original doc and the conflict doc differ.
        // I think temp_doc and conf_doc management should be handled in the Ledger and not the
        // Document struct
        let original_doc = self.get_doc(entry_name)?;
        let original_uuid = original_doc.get_uuid();
        let key = self.pw.expose_secret().clone();
        let mut loaded_conf_doc: Document<T> = decrypt_load_conf(&self.name, &key, entry_name)?;
        let conf_doc_uuid = loaded_conf_doc.get_uuid();

        if keep_original {
            self.remove(loaded_conf_doc.read_uuid())?;
        } else {
            loaded_conf_doc.append_uuid(&original_uuid);
            self.remove(&original_uuid)?;
            self.remove(&conf_doc_uuid)?;
            encrypt_store_doc(&mut loaded_conf_doc, &key)?;
        }

        Ok(())
    }

    pub fn read_by_entry_name<'a>(
        &'a mut self,
        entry_name: &str,
    ) -> Result<&'a T, LocalLedgerError> {
        self.read(entry_name.to_string())
    }

    /// Updates document for given `entry_name` with given `data`
    pub fn update(&mut self, entry_name: &str, data: T) -> Result<(), LocalLedgerError> {
        let entry_exists = self.entry_name_already_in_use(entry_name)?;

        if !entry_exists {
            return Err(LocalLedgerError::new("Entry name not found."));
        }

        let doc_is_cached = self.doc_cache.contains(entry_name); // entry_name_already_in_use does a cache check, and we do another on this line.  we should fix this later lol
        let key = &self.pw.expose_secret();

        if doc_is_cached {
            let mut cached_doc = self.doc_cache.get_mut(entry_name).map_or(
                Err(LocalLedgerError::new("Failed to get doc from cached")),
                |d| Ok(d),
            )?;

            if !cached_doc.has_been_decrypted() {
                decrypt_load_doc(cached_doc, entry_name, key)?;
            }

            cached_doc.update(data);

            encrypt_store_doc(&mut cached_doc, &self.pw.expose_secret())?;

            return Ok(());
        }

        let mut doc = Document::<T>::new(&self.name);
        decrypt_load_doc(&mut doc, entry_name, &self.pw.expose_secret())?;
        doc.update(data);
        encrypt_store_doc(&mut doc, &self.pw.expose_secret())?;

        self.doc_cache.put(entry_name.to_owned(), doc);

        Ok(())
    }

    pub fn remove(&mut self, entry_name: &str) -> Result<(), LocalLedgerError> {
        let doc_is_cached = self.doc_cache.contains(entry_name);

        if doc_is_cached {
            let cached_doc = self
                .doc_cache
                .get_mut(entry_name)
                .ok_or(LocalLedgerError::new("Failed to get doc from cache"))?;

            cached_doc.remove()?;

            let _ = self.doc_cache.pop_entry(entry_name);
        } else {
            Document::<T>::remove_doc(&self.name, entry_name)?;
        }

        Ok(())
    }

    pub fn list_entry_labels(&self) -> Result<Vec<String>, LocalLedgerError> {
        let labels: Vec<String> = Document::<T>::get_all_uuids(&self.name)?
            .into_iter()
            .filter(|uuid| uuid != META_DOC_UUID)
            .collect();

        Ok(labels)
    }

    pub fn list_entries_with_conflicts(&self) -> Result<Vec<String>, LocalLedgerError> {
        let labels: Vec<_> = Document::<T>::get_all_conflict_uuids(&self.name)?;

        Ok(labels)
    }

    pub fn get_ledger_dir(&self) -> Result<PathBuf, LocalLedgerError> {
        self.meta_doc.get_data_dir()
    }

    /// Retrieves all ledger's contents into a Read implementation.  Each doc is separated by a `\n` char
    pub fn doc_dump(&self) -> Result<LedgerDump, LocalLedgerError> {
        let src_dir = self.get_ledger_dir()?;
        let ld = LedgerDump::new(src_dir).map_err(|msg| LocalLedgerError::new(&msg))?;

        Ok(ld)
    }

    pub async fn merge<S>(&mut self, mut s: S) -> Result<(), LocalLedgerError>
    where
        S: Stream<Item = Result<Value, Box<dyn std::error::Error>>> + Unpin,
    {
        // This method is pretty beefy.  Probably should clean it up at some point.
        // I suppose this is an apology to my future self or whoever is dumb enough
        // to work on this
        let mut meta_doc_has_been_stored = false;
        let mut temp_stored_uuids: Vec<String> = vec![];
        let mut conflict_uuids: Vec<String> = vec![];

        while let Some(item) = s.next().await {
            let val = item.map_err(|e| LocalLedgerError::new(&e.to_string()))?;
            let uuid = assert_str(&val["uuid"]).map_err(|e| LocalLedgerError::new(&e))?;

            tracing::info!("merging doc uuid: {}", &uuid);

            if uuid.as_str() == META_DOC_UUID {
                let mut incomming_meta_doc =
                    serde_json::from_value::<Document<LocalLedgerMetaData>>(val)
                        .map_err(|e| LocalLedgerError::new(&e.to_string()))?;
                let conflict = Document::<LocalLedgerMetaData>::check_for_conflict(
                    &self.meta_doc,
                    &incomming_meta_doc,
                );

                if conflict {
                    // Conflict in the meta doc probably means that decryption is most likely to
                    // fail for the new imported docs.  We need to clear everything out and notify
                    // the user.
                    tracing::warn!("Meta doc conflict detected.");
                    temp_stored_uuids
                        .into_iter()
                        .chain(conflict_uuids.into_iter())
                        .try_for_each(|uuid| Document::<T>::remove_doc(&self.name, &uuid))?;

                    return Err(LocalLedgerError::meta_doc_conflict(
                        "META_DOC conflict found during merge.",
                    ));
                }

                incomming_meta_doc.store()?;
                meta_doc_has_been_stored = true;

                continue;
            }

            let mut incomming_ledger_doc = serde_json::from_value::<Document<T>>(val)
                .map_err(|e| LocalLedgerError::new(&e.to_string()))?;
            let our_ledger_doc = self.get_doc(&uuid)?;
            let conflict = Document::<T>::check_for_conflict(our_ledger_doc, &incomming_ledger_doc);

            if conflict {
                // Mark document as conflict
                incomming_ledger_doc.conflict_store()?;
                conflict_uuids.push(incomming_ledger_doc.get_uuid());
                tracing::warn!("Conflict found!");
                continue;
            }

            tracing::info!("No conflict found.");

            if !meta_doc_has_been_stored {
                incomming_ledger_doc.temp_store()?;
                temp_stored_uuids.push(incomming_ledger_doc.get_uuid());
                continue;
            }

            let key = &self.pw.expose_secret();
            incomming_ledger_doc.decrypt(|encrypted_data| {
                 tracing::info!("In decrypt callback");
                 let decryptor = match age::Decryptor::new(&encrypted_data[..]).map_err(|err| {
                     tracing::error!("decryptor error: {:?}", err);
                     LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string()))
                 })? {
                     age::Decryptor::Passphrase(d) => Ok(d),
                     _ => Err(LocalLedgerError::new("Failed to decrypt. Received encrypted data that was secured by some means other than a passphrase."))
                 }?;

                 tracing::info!("decrypting data...");
                 let mut decrypted = vec![];
                 let mut reader = decryptor
                     .decrypt(&Secret::new(key.to_string()), None)
                     .map_err(|err| {
                         LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string()))
                     })?;
                 tracing::info!("decrypting data success");

                 reader.read_to_end(&mut decrypted).map_err(|err| {
                     LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string()))
                 })?;

                 Ok(decrypted)
             })?;

            encrypt_store_doc(&mut incomming_ledger_doc, key)?;
        }

        // successfully drained the stream
        // loop through docs that were temp stored using the temp_stored_uuids vector

        let temp_docs = decrypt_load_temp_docs::<T>(&self.name, self.pw.expose_secret())?;

        for mut temp_doc in temp_docs.into_iter() {
            let uuid = Document::<T>::temp_uuid_to_uuid(temp_doc.read_uuid())?;

            temp_doc.append_uuid(&uuid);

            encrypt_store_doc(&mut temp_doc, self.pw.expose_secret())?;
        }

        tracing::info!("Merge stream finished.");

        Ok(())
    }

    fn get_doc<'a>(&'a mut self, uuid: &str) -> Result<&'a Document<T>, LocalLedgerError> {
        //TODO got some dup code with the `read` method
        //Made this because i needed a method that retrieved the Doc struct.
        //The read method reads the data inside a Document sturct
        let doc_is_cached = self.doc_cache.contains(uuid);
        let key = &self.pw.expose_secret();

        if doc_is_cached {
            let mut cached_doc = self.doc_cache.get_mut(uuid).map_or(
                Err(LocalLedgerError::new("Failed to get doc from cache")),
                |d| Ok(d),
            )?;

            if !cached_doc.has_been_decrypted() {
                decrypt_load_doc(&mut cached_doc, &uuid, &key)?;
            }

            return Ok(cached_doc);
        }

        let mut loaded_doc = Document::<T>::new(&self.name);

        decrypt_load_doc(&mut loaded_doc, &uuid, &key)?;

        self.doc_cache.put(uuid.to_string(), loaded_doc);

        let cached_doc = self
            .doc_cache
            .get(uuid)
            .ok_or(LocalLedgerError::new("Failed to get doc from cache"))?;

        Ok(cached_doc)
    }

    fn entry_name_already_in_use(&self, entry_name: &str) -> Result<bool, LocalLedgerError> {
        let in_cache = self.doc_cache.contains(entry_name);

        if in_cache {
            return Ok(true);
        }

        Document::<T>::doc_exists(&self.name, entry_name)
    }
}

fn decrypt_load_doc<T: Clone + Serialize + DeserializeOwned + Default + Debug>(
    curr_doc: &mut Document<T>,
    uuid: &str,
    key: &str,
) -> Result<(), LocalLedgerError> {
    let loaded_doc = Document::<T>::decrypt_load(curr_doc.label(), uuid, |encrypted_data| {
        let decryptor = match age::Decryptor::new(&encrypted_data[..]).map_err(|err| {
            tracing::error!("decryptor error: {:?}", err);
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

fn decrypt_load_temp_docs<T: Clone + Serialize + DeserializeOwned + Default + Debug>(
    label: &str,
    key: &str,
) -> Result<Vec<Document<T>>, LocalLedgerError> {
    let temp_uuids = Document::<T>::get_all_temp_uuids(label)?;

    let decrypted_temp_docs: Result<Vec<_>, _>  = temp_uuids.into_iter().map(|uuid| {
        let loaded_doc = Document::<T>::decrypt_load(label, &uuid, |encrypted_data| {
            let decryptor = match age::Decryptor::new(&encrypted_data[..]).map_err(|err| {
                tracing::error!("decryptor error: {:?}", err);
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
        });

        loaded_doc
    }).collect();

    decrypted_temp_docs
}

fn decrypt_load_conf<T>(label: &str, key: &str, uuid: &str) -> Result<Document<T>, LocalLedgerError>
where
    T: Clone + Serialize + DeserializeOwned + Default + Debug,
{
    let loaded_doc = Document::<T>::decrypt_load_conf(label, uuid, |encrypted_data| {
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
    });

    loaded_doc
}

fn try_load_meta_doc(ledger_name: &str) -> Option<Document<LocalLedgerMetaData>> {
    match Document::<LocalLedgerMetaData>::load(ledger_name, META_DOC_UUID) {
        Ok(meta_doc) => Some(meta_doc),
        Err(_err) => None,
    }
}

fn create_meta_doc(ledger_name: &str) -> Document<LocalLedgerMetaData> {
    let mut meta_doc = Document::<LocalLedgerMetaData>::new(ledger_name);

    meta_doc.append_uuid(META_DOC_UUID);

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
