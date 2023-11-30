use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::core::{Error, FSTORE};

#[derive(PartialEq, Eq, Copy, Clone)]
pub(crate) enum DirEntryType {
    File,
    Dir,
}

impl PartialOrd for DirEntryType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::*;
        match (self, other) {
            (DirEntryType::File, DirEntryType::File) => Some(Equal),
            (DirEntryType::File, DirEntryType::Dir) => Some(Greater),
            (DirEntryType::Dir, DirEntryType::File) => Some(Less),
            (DirEntryType::Dir, DirEntryType::Dir) => Some(Equal),
        }
    }
}

impl Ord for DirEntryType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        match (self, other) {
            (DirEntryType::File, DirEntryType::File) => Equal,
            (DirEntryType::File, DirEntryType::Dir) => Greater,
            (DirEntryType::Dir, DirEntryType::File) => Less,
            (DirEntryType::Dir, DirEntryType::Dir) => Equal,
        }
    }
}

pub(crate) struct DirEntry {
    depth: usize,
    entry_type: DirEntryType,
    name: OsString,
}

impl DirEntry {
    pub fn name(&self) -> &OsStr {
        &self.name
    }
}

pub(crate) struct WalkDirectories {
    cur_path: PathBuf,
    stack: Vec<DirEntry>,
    cur_depth: usize,
    num_children: usize,
}

impl WalkDirectories {
    pub fn from(dirpath: PathBuf) -> Result<Self, Error> {
        if !dirpath.is_dir() {
            return Err(Error::InvalidPath(dirpath));
        }
        Ok(WalkDirectories {
            cur_path: dirpath,
            stack: vec![DirEntry {
                depth: 1,
                entry_type: DirEntryType::Dir,
                name: OsString::from(""),
            }],
            cur_depth: 0,
            num_children: 0,
        })
    }

    pub(crate) fn next<'a>(&'a mut self) -> Option<(usize, &'a Path, &'a [DirEntry])> {
        while let Some(DirEntry {
            depth,
            entry_type,
            name,
        }) = self.stack.pop()
        {
            match entry_type {
                DirEntryType::File => continue,
                DirEntryType::Dir => {
                    while self.cur_depth > depth - 1 {
                        self.cur_path.pop();
                        self.cur_depth -= 1;
                    }
                    self.cur_path.push(name);
                    self.cur_depth += 1;
                    // Push all children.
                    let mut numfiles = 0;
                    let before = self.stack.len();
                    if let Ok(entries) = std::fs::read_dir(&self.cur_path) {
                        for entry in entries {
                            if let Ok(child) = entry {
                                let cname = child.file_name();
                                if cname.to_str().unwrap_or("") == FSTORE {
                                    continue;
                                }
                                match child.file_type() {
                                    Ok(ctype) => {
                                        if ctype.is_dir() {
                                            self.stack.push(DirEntry {
                                                depth: depth + 1,
                                                entry_type: DirEntryType::Dir,
                                                name: cname,
                                            });
                                        } else if ctype.is_file() {
                                            self.stack.push(DirEntry {
                                                depth: depth + 1,
                                                entry_type: DirEntryType::File,
                                                name: cname,
                                            });
                                            numfiles += 1;
                                        }
                                    }
                                    Err(_) => continue,
                                }
                            }
                        }
                    }
                    self.num_children = self.stack.len() - before;
                    let children = &mut self.stack[before..];
                    children.sort_by_key(|d| d.entry_type);
                    let children = &self.stack[(self.stack.len() - numfiles)..];
                    return Some((depth, &self.cur_path, children));
                }
            }
        }
        return None;
    }
}
