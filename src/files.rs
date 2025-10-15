use std::collections::{HashSet, VecDeque};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

// Attempt at fixing that last bit of performance, but does not change wall
// clock time in my /etc tests.
const BUFSIZ: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecurseHaystacks {
    No,
    FollowDirectories,
    FollowDirectorySymlinks,
}

pub struct FileSource {
    pub name: String,
    pub reader: Box<dyn BufRead>,
}

enum FileEntry {
    Stdin,
    FollowPath(PathBuf),
    NoFollowPath(PathBuf),
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
struct DirId {
    dev: u64,
    ino: u64,
}

pub struct FileSourceIter {
    stack: VecDeque<FileEntry>,
    recurse: RecurseHaystacks,
    dirs_seen: HashSet<DirId>,
}

#[allow(clippy::new_without_default)]
impl FileSourceIter {
    /// Create an empty iterator builder.
    pub fn new() -> Self {
        FileSourceIter {
            stack: VecDeque::new(),
            recurse: RecurseHaystacks::No,
            dirs_seen: HashSet::<DirId>::new(),
        }
    }

    /// Awkward check to see whether we have more than one file, for
    /// show-filename purposes.
    pub fn has_more_than_one_file(&self) -> bool {
        if self.stack.len() > 1 {
            return true;
        }
        match self.stack.front() {
            Some(FileEntry::Stdin) | None => false,
            Some(FileEntry::FollowPath(path)) => {
                if let Ok(stat) = fs::metadata(path) {
                    stat.is_dir()
                } else {
                    false
                }
            }
            Some(FileEntry::NoFollowPath(path)) => {
                if let Ok(stat) = fs::symlink_metadata(path) {
                    stat.is_dir()
                } else {
                    false
                }
            }
        }
    }

    /// Enable or disable recursion.
    pub fn set_recursion(mut self, recurse: RecurseHaystacks) -> Self {
        self.recurse = recurse;
        self
    }

    /// Add stdin ("-") to the stack.
    pub fn add_stdin(mut self) -> Self {
        self.stack.push_back(FileEntry::Stdin);
        self
    }

    /// Add a list of files or directories to the stack.
    pub fn add_files<I, S>(mut self, files: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for f in files {
            self.stack
                .push_back(FileEntry::FollowPath(PathBuf::from(f.as_ref())));
        }
        self
    }

    /// Iterator implementation.
    fn next_impl(&mut self) -> Option<<Self as Iterator>::Item> {
        while let Some(entry) = self.stack.pop_front() {
            match entry {
                FileEntry::Stdin => {
                    if let Some(item) = self.next_stdin() {
                        return Some(item);
                    }
                }
                FileEntry::FollowPath(ref path) => {
                    if let Some(item) = self.next_path(path, true) {
                        return Some(item);
                    }
                }
                FileEntry::NoFollowPath(ref path) => {
                    if let Some(item) = self.next_path(path, false) {
                        return Some(item);
                    }
                }
            }
        }
        None
    }

    /// Return Stdin file handle as item.
    fn next_stdin(&self) -> Option<<Self as Iterator>::Item> {
        Some(Ok(FileSource {
            name: "(stdin)".into(),
            reader: Box::new(BufReader::with_capacity(BUFSIZ, io::stdin())),
        }))
    }

    /// Return real file handle as item (or None while populating dirs).
    fn next_path(
        &mut self,
        path: &PathBuf,
        follow: bool,
    ) -> Option<<Self as Iterator>::Item> {
        let stat = match match follow {
            true => fs::metadata(path),          // stat
            false => fs::symlink_metadata(path), // lstat
        } {
            Ok(m) => m,
            Err(e) => {
                return Some(Err(format!("{}: {e}", path.display())));
            }
        };

        if stat.is_symlink() {
            self.next_path_symlink()
        } else if stat.is_dir() {
            let dir_id = DirId {
                dev: stat.dev(),
                ino: stat.ino(),
            };
            if self.dirs_seen.insert(dir_id) {
                self.next_path_dir(path)
            } else {
                eprintln!(
                    "ipgrep: {}: warning: recursive directory loop",
                    path.display()
                );
                None
            }
        } else {
            self.next_path_file(path)
        }
    }

    /// Return None because we either follow symlinks or ignore them.
    fn next_path_symlink(&self) -> Option<<Self as Iterator>::Item> {
        // Handle symlinks by ignoring them. is_symlink() will only return true
        // if we used symlink_metadata (follow == false), and then we do not
        // want to follow them.
        //
        // Behave like GNU grep 3.7:
        //
        // $ ls -1
        // hello -> hello.txt (symlink)
        // hello.txt
        // nohello -> nohello.txt (symlink)
        // nohello.txt
        //
        // $ grep nohello . -cr
        // ./hello.txt:0
        // ./nohello.txt:1
        //
        // $ grep nohello . -cR
        // ./hello:0
        // ./hello.txt:0
        // ./nohello:1
        // ./nohello.txt:1
        None // silently continue to next
    }

    /// Return no file handle, but fill the stack with new files/directories.
    fn next_path_dir(
        &mut self,
        path: &PathBuf,
    ) -> Option<<Self as Iterator>::Item> {
        if self.recurse == RecurseHaystacks::No {
            return Some(Err(format!("{}: Is a directory", path.display())));
        }

        match fs::read_dir(path) {
            Ok(entries) => {
                // GNU grep 3.11 does not sort the files. We don't either.
                // let mut entries: Vec<_> = entries.flatten().collect();
                // entries.sort_by_key(|e| e.file_name());
                for entry in entries.flatten() {
                    let child_path = entry.path();
                    self.stack.push_back(match self.recurse {
                        RecurseHaystacks::FollowDirectories => {
                            FileEntry::NoFollowPath(child_path)
                        }
                        RecurseHaystacks::FollowDirectorySymlinks => {
                            FileEntry::FollowPath(child_path)
                        }
                        RecurseHaystacks::No => unreachable!(),
                    });
                }

                // Done populating more directories. Go back and let
                // the main iterator loop find a file.
                None
            }
            Err(e) => Some(Err(format!("{}: {e}", path.display()))),
        }
    }

    /// Return real file handle.
    fn next_path_file(
        &mut self,
        path: &PathBuf,
    ) -> Option<<Self as Iterator>::Item> {
        match File::open(path) {
            Ok(f) => Some(Ok(FileSource {
                name: path.display().to_string(),
                reader: Box::new(BufReader::with_capacity(BUFSIZ, f)),
            })),
            Err(e) => Some(Err(format!("{}: {e}", path.display()))),
        }
    }
}

impl Iterator for FileSourceIter {
    type Item = Result<FileSource, String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl()
    }
}
