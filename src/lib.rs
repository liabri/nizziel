mod blocking;

mod error;
use error::Error;

use std::io::{ Seek, SeekFrom, Write };
use std::path::{ Path, PathBuf };
use std::fs::{ create_dir_all, File };
use futures::StreamExt;

#[derive(Clone, Default, Debug)]
pub struct Downloads {
    pub downloads: Vec<Download>,
    pub retries: u8,
}

#[derive(Clone, Default, Debug)]
pub struct Download {
    pub url: String,
    pub path: PathBuf,
    pub unzip: bool,
}

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