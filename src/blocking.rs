use crate::{ create_file, Error };
use std::path::Path;
use std::io::Write;

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