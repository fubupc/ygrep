use std::{fs, io, path};

/// Iterate recursively over files under a path.
pub enum WalkPath {
    File(path::PathBuf),
    Dir(WalkDir),
    Done,
}

/// Iterate recursively over (only) files in a directory.
pub struct WalkDir {
    follow_symlink: bool,
    path: path::PathBuf,
    inner: fs::ReadDir,
    child_iter: Option<Box<WalkDir>>,
}

/// WalkPath/WalkDir iteration error
pub struct Error {
    pub path: path::PathBuf,
    pub err: io::Error,
}

impl Iterator for WalkPath {
    type Item = Result<path::PathBuf, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            WalkPath::File(p) => {
                let p = p.clone();
                *self = WalkPath::Done;
                Some(Ok(p))
            }
            WalkPath::Dir(wd) => wd.next(),
            WalkPath::Done => None,
        }
    }
}

impl Iterator for WalkDir {
    type Item = Result<path::PathBuf, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // Helper function returns Result<Option<_>, _> to use `?` operator happily.
        fn throw_err(wd: &mut WalkDir) -> Result<Option<path::PathBuf>, Error> {
            while let Some(e) = wd.inner.next() {
                let e = e.map_err(|err| Error::new(err, wd.path.clone()))?;
                let meta = if wd.follow_symlink {
                    fs::metadata(e.path())
                } else {
                    fs::symlink_metadata(e.path())
                };
                let meta = meta.map_err(|err| Error::new(err, e.path()))?;

                if meta.is_file() {
                    return Ok(Some(e.path()));
                }

                if meta.is_dir() {
                    let mut child_iter = walk_dir(e.path(), wd.follow_symlink)
                        .map_err(|err| Error::new(err, e.path()))?;
                    match child_iter.next() {
                        Some(p) => {
                            let p = p?;
                            wd.child_iter = Some(Box::new(child_iter));
                            return Ok(Some(p));
                        }
                        None => continue,
                    }
                }

                // meta.is_symlink() must be false
            }
            Ok(None)
        }

        // Return next item of child dir being iterated over until it's drained.
        if let Some(child_iter) = &mut self.child_iter {
            if let Some(e) = child_iter.next() {
                return Some(e);
            }
            self.child_iter = None;
        }
        match throw_err(self) {
            Ok(None) => None,
            Ok(Some(p)) => Some(Ok(p)),
            Err(e) => Some(Err(e)),
        }
    }
}

impl Error {
    pub fn new(err: io::Error, path: path::PathBuf) -> Self {
        Self { path, err }
    }
}

pub fn walk_path<P: AsRef<path::Path>>(path: P, follow_symlink: bool) -> io::Result<WalkPath> {
    // `follow_symlink` does not apply to `path` itself
    let meta = fs::metadata(&path)?;
    if meta.is_file() {
        return Ok(WalkPath::File(path.as_ref().to_path_buf()));
    }
    if meta.is_dir() {
        return Ok(WalkPath::Dir(WalkDir {
            path: path.as_ref().to_path_buf(),
            inner: fs::read_dir(&path)?,
            child_iter: None,
            follow_symlink,
        }));
    }
    unreachable!(
        "path (after following symlink) `{}` must be either file or dir",
        path.as_ref().display()
    )
}

pub fn walk_dir<P: AsRef<path::Path>>(path: P, follow_symlink: bool) -> io::Result<WalkDir> {
    Ok(WalkDir {
        follow_symlink,
        path: path.as_ref().to_path_buf(),
        inner: fs::read_dir(&path)?,
        child_iter: None,
    })
}
