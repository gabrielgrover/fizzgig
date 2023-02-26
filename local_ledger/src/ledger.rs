use age::secrecy::Secret;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Read, Write},
};

use pwhash::bcrypt;

use crate::{
    utility::{generate_id, LocalLedgerError},
    Document,
};

#[derive(Debug, Clone)]
pub struct LocalLedger<T> {
    name: String,
    uuid: String,
    encrypted: bool,
    key: String,
    doc_cache: HashMap<String, Document<T>>,
}

// How do we ensure encrypted data has not been tampered with?

impl<T> LocalLedger<T>
where
    T: Clone + Serialize + DeserializeOwned + Default + Debug,
{
    pub fn new(name: &str, ledger_password: String) -> Result<Self, LocalLedgerError> {
        let key = bcrypt::hash(&ledger_password).map_err(|err| {
            LocalLedgerError::new(&format!("LocalLedger creation failed: {}", err.to_string()))
        })?;

        Ok(LocalLedger {
            name: name.to_owned(),
            uuid: generate_id(),
            encrypted: true,
            key,
            doc_cache: HashMap::new(),
        })
    }

    /// Writes to the ledger.  Returning a uuid.
    pub fn write(&mut self, data: T) -> Result<String, LocalLedgerError> {
        let encrypted_doc = Document::<T>::new(&self.name)
            .update(data)
            .store_encrypted(|data| {
                let encryptor =
                    age::Encryptor::with_user_passphrase(Secret::new(self.key.to_owned()));
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

        let doc_uuid = encrypted_doc.get_uuid();

        self.doc_cache.insert(doc_uuid.clone(), encrypted_doc);

        Ok(doc_uuid)
    }

    pub fn read<'a>(&'a mut self, uuid: String) -> Result<&'a T, LocalLedgerError> {
        let doc = Document::<T>::new(&self.name).decrypt_load(&uuid, |encrypted_data| {
            let decryptor = match age::Decryptor::new(&encrypted_data[..]).map_err(|err| {
                LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string()))
            })? {

                age::Decryptor::Passphrase(d) => Ok(d),

                _ => Err(LocalLedgerError::new("Failed to decrypt.  Received encrypted data that was secured by some means other than a passphrase.")),
            }?;

            let mut decrypted = vec![];
            let mut reader = decryptor
                .decrypt(&Secret::new(self.key.to_owned()), None)
                .map_err(|err| {
                    LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string()))
                })?;

            reader.read_to_end(&mut decrypted).map_err(|err| LocalLedgerError::new(&format!("Failed to decrypt data: {}", err.to_string())))?;

            Ok(decrypted)
        })?;

        self.doc_cache.insert(uuid.clone(), doc);

        self.doc_cache.get(&uuid).unwrap().read_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
    struct Person {
        age: i32,
        name: String,
    }

    #[test]
    fn should_be_able_to_write_to_the_ledger() {
        let person = Person {
            age: 21,
            name: "duderino".to_owned(),
        };

        let mut user_ledger = LocalLedger::<Person>::new("Users", "password".to_owned()).unwrap();

        user_ledger
            .write(person.clone())
            .expect("Failed to write to ledger");
    }

    #[test]
    fn should_be_able_to_read_ledger() {
        let person = Person {
            age: 21,
            name: "duderino".to_owned(),
        };

        let mut user_ledger = LocalLedger::<Person>::new("Users", "password".to_owned()).unwrap();

        let uuid = user_ledger
            .write(person.clone())
            .expect("Failed to write to ledger");

        let received_data = user_ledger.read(uuid).unwrap();

        assert_eq!(received_data, &person);
    }
}
