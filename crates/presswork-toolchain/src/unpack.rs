use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Component, Path, PathBuf};

use crate::error::ToolchainError;

const XZ_MEMORY_LIMIT_KB: u32 = 256 * 1024;

pub(crate) fn unpack(archive: &Path, destination: &Path) -> Result<(), ToolchainError> {
    let name = archive
        .file_name()
        .map_or_default(|name| name.to_string_lossy().into_owned());
    if has_suffix(&name, ".zip") {
        unpack_zip(archive, destination)
    } else if has_suffix(&name, ".tar.xz") {
        let xz = lzma_rust2::XzReader::new_mem_limit(
            BufReader::new(open(archive)?),
            false,
            XZ_MEMORY_LIMIT_KB,
        );
        unpack_tar(xz, destination)
    } else if has_suffix(&name, ".tar.gz") || has_suffix(&name, ".tgz") {
        let gz = flate2::read::GzDecoder::new(BufReader::new(open(archive)?));
        unpack_tar(gz, destination)
    } else {
        Err(ToolchainError::UnsupportedArchive(name))
    }
}

fn has_suffix(name: &str, suffix: &str) -> bool {
    name.len()
        .checked_sub(suffix.len())
        .and_then(|start| name.get(start..))
        .is_some_and(|end| end.eq_ignore_ascii_case(suffix))
}

fn unpack_zip(archive: &Path, destination: &Path) -> Result<(), ToolchainError> {
    let mut zip = zip::ZipArchive::new(open(archive)?).map_err(|error| broken(&error))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| broken(&error))?;
        let name = entry.name().map_err(|error| broken(&error))?.into_owned();
        if entry.is_symlink() {
            return Err(ToolchainError::UnsafeArchiveEntry(name));
        }
        let target = destination.join(inside(&name)?);
        if entry.is_dir() {
            create_folder(&target)?;
        } else {
            let mode = entry.unix_mode();
            write_file(&target, &mut entry, mode)?;
        }
    }
    Ok(())
}

fn unpack_tar(data: impl Read, destination: &Path) -> Result<(), ToolchainError> {
    let mut tar = tar::Archive::new(data);
    for entry in tar.entries().map_err(|error| broken(&error))? {
        let mut entry = entry.map_err(|error| broken(&error))?;
        let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        let target = destination.join(inside(&name)?);
        match entry.header().entry_type() {
            tar::EntryType::Directory => create_folder(&target)?,
            tar::EntryType::Regular | tar::EntryType::Continuous => {
                let mode = entry.header().mode().ok();
                write_file(&target, &mut entry, mode)?;
            }
            tar::EntryType::XGlobalHeader => {}
            _ => return Err(ToolchainError::UnsafeArchiveEntry(name)),
        }
    }
    Ok(())
}

fn inside(name: &str) -> Result<PathBuf, ToolchainError> {
    let mut path = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ToolchainError::UnsafeArchiveEntry(name.to_owned()));
            }
        }
    }
    Ok(path)
}

fn open(path: &Path) -> Result<File, ToolchainError> {
    File::open(path).map_err(|source| ToolchainError::Io {
        path: path.to_owned(),
        source,
    })
}

fn create_folder(path: &Path) -> Result<(), ToolchainError> {
    fs::create_dir_all(path).map_err(|source| ToolchainError::Io {
        path: path.to_owned(),
        source,
    })
}

fn write_file(
    target: &Path,
    data: &mut impl Read,
    mode: Option<u32>,
) -> Result<(), ToolchainError> {
    if let Some(parent) = target.parent() {
        create_folder(parent)?;
    }
    let io_error = |source| ToolchainError::Io {
        path: target.to_owned(),
        source,
    };
    let mut file = File::create(target).map_err(io_error)?;
    io::copy(data, &mut file).map_err(io_error)?;
    set_mode(target, mode)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: Option<u32>) -> Result<(), ToolchainError> {
    use std::os::unix::fs::PermissionsExt;
    let Some(mode) = mode else {
        return Ok(());
    };
    fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o755)).map_err(|source| {
        ToolchainError::Io {
            path: path.to_owned(),
            source,
        }
    })
}

#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the Unix version can fail, and both versions have one signature"
)]
fn set_mode(_path: &Path, _mode: Option<u32>) -> Result<(), ToolchainError> {
    Ok(())
}

fn broken(error: &dyn std::fmt::Display) -> ToolchainError {
    ToolchainError::BrokenArchive(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{tar_gz_file, tar_xz_file, zip_file};

    #[test]
    fn a_zip_file_unpacks_into_the_folder() {
        let folder = tempfile::tempdir().unwrap();
        let archive = zip_file(
            folder.path(),
            &[
                ("typst-x/typst.exe", b"binary"),
                ("typst-x/LICENSE", b"Apache"),
            ],
        );
        let output = folder.path().join("output");

        unpack(&archive, &output).unwrap();

        assert_eq!(
            std::fs::read(output.join("typst-x/typst.exe")).unwrap(),
            b"binary"
        );
        assert_eq!(
            std::fs::read(output.join("typst-x/LICENSE")).unwrap(),
            b"Apache"
        );
    }

    #[test]
    fn a_tar_gz_file_unpacks_into_the_folder() {
        let folder = tempfile::tempdir().unwrap();
        let archive = tar_gz_file(
            folder.path(),
            "package.tar.gz",
            &[
                ("typst.toml", tar::EntryType::Regular, b"[package]"),
                ("src/", tar::EntryType::Directory, b""),
                ("src/lib.typ", tar::EntryType::Regular, b"#let x = 1"),
            ],
        );
        let output = folder.path().join("output");

        unpack(&archive, &output).unwrap();

        assert_eq!(
            std::fs::read(output.join("typst.toml")).unwrap(),
            b"[package]"
        );
        assert_eq!(
            std::fs::read(output.join("src/lib.typ")).unwrap(),
            b"#let x = 1"
        );
    }

    #[test]
    fn a_tar_gz_entry_that_leaves_the_folder_stops_the_unpacking() {
        let folder = tempfile::tempdir().unwrap();
        let archive = tar_gz_file(
            folder.path(),
            "package.tar.gz",
            &[("../escape", tar::EntryType::Regular, b"hostile")],
        );

        let result = unpack(&archive, &folder.path().join("output"));

        assert!(matches!(result, Err(ToolchainError::UnsafeArchiveEntry(_))));
        assert!(!folder.path().join("escape").exists());
    }

    #[test]
    fn a_tar_xz_file_unpacks_into_the_folder() {
        let folder = tempfile::tempdir().unwrap();
        let archive = tar_xz_file(
            folder.path(),
            &[
                ("./", tar::EntryType::Directory, b""),
                ("typst-x/", tar::EntryType::Directory, b""),
                ("typst-x/typst", tar::EntryType::Regular, b"binary"),
            ],
        );
        let output = folder.path().join("output");

        unpack(&archive, &output).unwrap();

        let executable = output.join("typst-x/typst");
        assert_eq!(std::fs::read(&executable).unwrap(), b"binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&executable).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111);
        }
    }

    #[test]
    fn an_entry_that_leaves_the_folder_stops_the_unpacking() {
        for name in ["../escape", "typst-x/../../escape", "/escape"] {
            let folder = tempfile::tempdir().unwrap();
            let archive = tar_xz_file(
                folder.path(),
                &[(name, tar::EntryType::Regular, b"hostile")],
            );

            let result = unpack(&archive, &folder.path().join("output"));

            assert!(
                matches!(&result, Err(ToolchainError::UnsafeArchiveEntry(entry)) if entry == name),
                "{name}: {result:?}"
            );
            assert!(!folder.path().join("escape").exists());
        }
    }

    #[test]
    fn a_zip_entry_that_leaves_the_folder_stops_the_unpacking() {
        let folder = tempfile::tempdir().unwrap();
        let archive = zip_file(folder.path(), &[("../escape", b"hostile")]);

        let result = unpack(&archive, &folder.path().join("output"));

        assert!(matches!(result, Err(ToolchainError::UnsafeArchiveEntry(_))));
        assert!(!folder.path().join("escape").exists());
    }

    #[test]
    fn a_link_stops_the_unpacking() {
        let folder = tempfile::tempdir().unwrap();
        let archive = tar_xz_file(
            folder.path(),
            &[("typst-x/link", tar::EntryType::Symlink, b"")],
        );

        let result = unpack(&archive, &folder.path().join("output"));

        assert!(
            matches!(&result, Err(ToolchainError::UnsafeArchiveEntry(entry)) if entry == "typst-x/link")
        );
    }

    #[test]
    fn a_file_that_is_not_an_archive_gives_an_error() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("typst.7z");
        std::fs::write(&path, "7z").unwrap();

        assert!(matches!(
            unpack(&path, &folder.path().join("output")),
            Err(ToolchainError::UnsupportedArchive(name)) if name == "typst.7z"
        ));
    }
}
