use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn zip_file(folder: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = folder.join("typst.zip");
    let mut writer = zip::ZipWriter::new(File::create(&path).unwrap());
    for (name, data) in entries {
        writer
            .start_file(
                *name,
                zip::write::SimpleFileOptions::default().unix_permissions(0o755),
            )
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
    path
}

pub(crate) fn tar_xz_file(folder: &Path, entries: &[(&str, tar::EntryType, &[u8])]) -> PathBuf {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, kind, data) in entries {
        let mut header = tar::Header::new_gnu();
        header.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
        header.set_entry_type(*kind);
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append(&header, *data).unwrap();
    }
    let path = folder.join("typst.tar.xz");
    let mut writer = lzma_rust2::XzWriter::new(
        File::create(&path).unwrap(),
        lzma_rust2::XzOptions::with_preset(1),
    )
    .unwrap();
    writer.write_all(&builder.into_inner().unwrap()).unwrap();
    writer.finish().unwrap();
    path
}
