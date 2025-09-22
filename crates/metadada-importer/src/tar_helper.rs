use anyhow::Result;
use bzip2::bufread::BzDecoder;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
use tar::Archive;

pub fn get_archive(tmpfile: &Path) -> Result<Archive<impl Read>> {
    let f = File::open(tmpfile)?;
    let reader = BufReader::new(f);
    let decompressor = BzDecoder::new(reader);
    let archive = Archive::new(decompressor);
    Ok(archive)
}
