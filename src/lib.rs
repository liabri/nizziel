use std::io::{ Seek, SeekFrom, Write };
use std::path::{ Path, PathBuf };
use std::fs::{ File, create_dir_all };

#[derive(Clone, Default, Debug)]
pub struct Downloads {
    pub downloads: Vec<Download>,
    pub retries: u8,
}

impl std::error::Error for Error {}

#[derive(Clone, Default, Debug)]
pub struct Download {
    pub url: String,
    pub path: PathBuf,
    pub unzip: bool,
}

#[derive(Debug)]
pub enum Error {
    Temp
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &*self {
            Temp => f.write_str("temp")
        }
    }
}

//shouldnt extract, just return the ZipArchive

pub fn create_file(path: &Path, read: bool, write: bool) -> std::io::Result<File> {
    let mut file = std::fs::OpenOptions::new()
        .read(read)
        .write(write)
        .open(path);

    if let Err(_) = file {
        create_dir_all(&path.parent().unwrap()).unwrap(); //handle non-existence of parent()

        file = std::fs::OpenOptions::new()
            .read(read)
            .write(write)
            .create(true)
            .open(path);
    }

    Ok(file.unwrap())
}

use futures::StreamExt;

pub async fn download(dls: Downloads) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let retries = dls.retries.clone();

    let fetches = futures::stream::iter(    
        dls.downloads.iter().map(|download| {
            let client = client.clone();

            async move {
                if download.unzip {
                    create_dir_all(&download.path).unwrap();

                    for _ in 1..=retries {
                        if reqwest::StatusCode::from_u16({ 
                            if let Ok(response) = client.get(&download.url).send().await {
                                let status = response.status().as_u16();
                                unzip(&response.bytes().await.unwrap().to_vec(), &download.path).await;
                                status
                            } else {
                                reqwest::StatusCode::BAD_REQUEST.as_u16()
                            }
                        }).unwrap_or(reqwest::StatusCode::BAD_REQUEST).is_success() {
                            break;
                        }
                    }
                } else {
                    let file = create_file(&download.path, true, true).unwrap();
                    let mut writer = std::io::BufWriter::new(file);

                    for _ in 1..=retries {
                        if reqwest::StatusCode::from_u16({ 
                            if let Ok(mut response) = client.get(&download.url).send().await {
                                let status = response.status().as_u16();
                                let mut current: u64 = 0;

                                writer.seek(SeekFrom::Start(current)).unwrap_or(0);
                                while let Some(bytes) = response.chunk().await.unwrap_or(None) {
                                    writer.write_all(&bytes).unwrap();
                                    current += bytes.len() as u64;
                                }

                                status
                            } else {
                                reqwest::StatusCode::BAD_REQUEST.as_u16()
                            }
                        }).unwrap_or(reqwest::StatusCode::BAD_REQUEST).is_success() {
                            break;
                        }
                    }    
                }
            }
        })
    ).buffer_unordered(100).collect::<Vec<()>>();

    fetches.await;
    Ok(())
}

pub async fn unzip(bytes: &[u8], path: &Path) {
    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let path = format!("{}/", path.to_str().unwrap());
    zip.extract(path).unwrap();
}

pub mod blocking {
    use crate::{ create_file, Error };
    use std::io::{ Seek, SeekFrom, Write };
    use std::fs::{ File, create_dir_all };
    use std::path::{ Path, PathBuf };

    pub fn download(url: &str, path: &Path, dezip: bool) -> Result<Vec<u8>, Error> {
        let mut file = create_file(&path, true, true).unwrap();
        let resp = reqwest::blocking::get(url).unwrap();
        let bytes = resp.bytes().unwrap();
        file.write_all(&bytes).unwrap();

        if dezip {
            unzip(&bytes, path).unwrap();
        }

        Ok(bytes.to_vec())
    }

    fn unzip(bytes: &[u8], path: &Path) -> Result<(), Error> {
        let reader = std::io::Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(reader).unwrap();
        let path = format!("{}/", path.to_str().unwrap());
        zip.extract(path).unwrap();
        Ok(())
    }
}