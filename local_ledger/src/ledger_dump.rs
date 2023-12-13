use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::PathBuf,
};

use walkdir::DirEntry;

/// Implements Reader in order to give a byte stream of ledger's contents.  Each document is separated by a `\n` char
pub struct LedgerDump {
    entries: Vec<Result<DirEntry, walkdir::Error>>,
    current_reader: Option<BufReader<File>>,
    eof_reached: bool,
}

impl LedgerDump {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let mut entries = vec![];
        let path_is_dir = fs::metadata(&path).map_err(|e| e.to_string())?.is_dir();

        for entry in walkdir::WalkDir::new(path) {
            entries.push(entry);
        }

        if path_is_dir {
            // if path is a dir the walkdir places it as the first entry
            // so we need to remove it since we want a list of files only
            let _r = entries.remove(0);
        }

        Ok(Self {
            entries,
            current_reader: None,
            eof_reached: false,
        })
    }
}

impl Read for LedgerDump {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.current_reader.is_none() && self.entries.len() == 0 {
            return Ok(0);
        }

        let mut reader = match self.current_reader.take() {
            Some(current_reader) => current_reader,

            None => {
                let entry = self.entries.remove(0)?;
                let path = entry.path();
                let file = File::open(path)?;

                BufReader::new(file)
            }
        };

        if self.eof_reached {
            let len = buf.len().min(1);
            buf[..len].copy_from_slice(&[b'\n']);
            self.current_reader = Some(reader);
            self.eof_reached = false;

            return Ok(1);
        }

        let bytes_read = reader.read(buf)?;

        if bytes_read <= 0 {
            self.eof_reached = true;
            self.current_reader = None;
        } else {
            self.current_reader = Some(reader);
        }

        Ok(bytes_read)
    }
}
