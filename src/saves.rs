//! The save list: each save is one game of one library, `saves/<n>.json` in the data
//! directory, numbered in the order they were begun. Before the list each library
//! had one save beside the settings; the first run with it copies those in
//! (`import_old`).

use crate::game::SaveSummary;
use crate::library::Layout;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct Save {
    pub path: PathBuf,
    /// When it was last written.
    pub when: SystemTime,
    pub summary: SaveSummary,
}

fn dir(data: &Path) -> PathBuf {
    data.join("saves")
}

/// The number a save file is named by (`7.json`, and `7.old.json` set aside from it).
fn number(name: &std::ffi::OsStr) -> Option<u32> {
    name.to_str()?.split('.').next()?.parse().ok()
}

/// The saves, the one written last first. Files that are not a save (one set aside
/// as from another version, `<n>.old.json`, or one cut short) are passed over.
pub fn list(data: &Path) -> Vec<Save> {
    let Ok(entries) = std::fs::read_dir(dir(data)) else {
        return Vec::new();
    };
    let mut saves: Vec<Save> = (entries.flatten())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter(|e| e.path().file_stem().and_then(number).is_some())
        .filter_map(|e| {
            Some(Save {
                when: e.metadata().ok()?.modified().ok()?,
                summary: SaveSummary::read(&e.path())?,
                path: e.path(),
            })
        })
        .collect();
    saves.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| b.path.cmp(&a.path)));
    saves
}

/// Where a new game is saved: numbered after every file in the list, so none of
/// them is ever written over.
pub fn new_path(data: &Path) -> PathBuf {
    let last = (std::fs::read_dir(dir(data)).into_iter().flatten().flatten())
        .filter_map(|e| number(&e.file_name()))
        .max()
        .unwrap_or(0);
    dir(data).join(format!("{}.json", last + 1))
}

/// Delete a save, and any copy of it set aside as from another version.
pub fn delete(path: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(path.with_extension("old.json"));
    std::fs::remove_file(path)
}

/// Each library's one save from before the list (the Hall's under its 0.1 name).
const OLD: [(&str, Layout); 2] = [("save.json", Layout::Hall), ("great.json", Layout::Great)];

/// The first run with the save list copies the saves from before it in, oldest first,
/// with their library written in and their times kept. The old files are left as
/// they were and, the list now being there, never read again.
pub fn import_old(data: &Path) {
    let dir = dir(data);
    if dir.exists() {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("hushfold: can't make the save list at {}: {e}", dir.display());
        return;
    }
    let mut old: Vec<(PathBuf, Layout, SystemTime)> = (OLD.iter())
        .filter_map(|&(name, layout)| {
            let path = data.join(name);
            let when = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((path, layout, when))
        })
        .collect();
    old.sort_by_key(|o| o.2);
    let mut n = 0;
    for (path, layout, when) in old {
        let read = std::fs::read(&path).ok();
        let Some(mut save) = read.and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) else {
            continue;
        };
        let Some(fields) = save.as_object_mut() else {
            continue;
        };
        fields.insert("layout".into(), serde_json::json!(layout));
        n += 1;
        let to = dir.join(format!("{n}.json"));
        let copied = serde_json::to_vec(&save)
            .map_err(std::io::Error::other)
            .and_then(|bytes| {
                std::fs::write(&to, bytes)?;
                std::fs::File::options().write(true).open(&to)?.set_modified(when)
            });
        match copied {
            Ok(()) => eprintln!("hushfold: {} is now {} in the save list", path.display(), to.display()),
            Err(e) => eprintln!("hushfold: couldn't copy {} into the save list: {e}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use std::time::Duration;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hushfold-saves-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn saved(layout: Layout, path: &Path, lumens: u64) -> Game {
        let mut g = Game::new(layout);
        g.save_path = Some(path.into());
        g.lumens = lumens;
        g.save();
        g
    }

    fn age(path: &Path, secs: u64) {
        let when = SystemTime::now() - Duration::from_secs(secs);
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(when)
            .unwrap();
    }

    /// The old saves come into the list as they were, the newer last; the files they
    /// came from are left alone, and a second run copies nothing again.
    #[test]
    fn the_saves_from_before_the_list_are_copied_in() {
        let data = scratch("import");
        let hall = saved(Layout::Hall, &data.join("save.json"), 7);
        let great = saved(Layout::Great, &data.join("great.json"), 9);
        // Saves from before the list didn't say which library they were.
        for name in ["save.json", "great.json"] {
            let path = data.join(name);
            let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            v.as_object_mut().unwrap().remove("layout");
            std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        }
        age(&data.join("save.json"), 3600);
        age(&data.join("great.json"), 60);
        let before: Vec<Vec<u8>> = ["save.json", "great.json"]
            .map(|n| std::fs::read(data.join(n)).unwrap())
            .into();
        import_old(&data);
        let list = list(&data);
        let got: Vec<(Layout, PathBuf)> = list.iter().map(|s| (s.summary.layout, s.path.clone())).collect();
        let dir = data.join("saves");
        assert_eq!(
            got,
            vec![(Layout::Great, dir.join("2.json")), (Layout::Hall, dir.join("1.json"))]
        );
        assert_eq!(
            list[0].when,
            std::fs::metadata(data.join("great.json")).unwrap().modified().unwrap()
        );
        for (old, s) in [(&great, &list[0]), (&hall, &list[1])] {
            let mut g = Game::new(old.layout);
            g.save_path = Some(s.path.clone());
            assert!(g.load());
            assert_eq!(g.lumens, old.lumens);
            assert_eq!(s.summary.books, old.total_books());
        }
        let after: Vec<Vec<u8>> = ["save.json", "great.json"]
            .map(|n| std::fs::read(data.join(n)).unwrap())
            .into();
        assert_eq!(before, after);
        std::fs::remove_file(dir.join("1.json")).unwrap();
        import_old(&data);
        assert_eq!(super::list(&data).len(), 1);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// A new game is numbered past every file in the list, kept aside or not; only
    /// saves are listed, newest first; deleting takes what was set aside too.
    #[test]
    fn the_list_numbers_orders_and_deletes() {
        let data = scratch("list");
        assert!(list(&data).is_empty());
        let first = new_path(&data);
        assert_eq!(first, data.join("saves/1.json"));
        saved(Layout::Hall, &first, 1);
        let second = new_path(&data);
        saved(Layout::Hall, &second, 2);
        age(&second, 600);
        std::fs::write(data.join("saves/3.old.json"), "{}").unwrap();
        std::fs::write(data.join("saves/2.old.json"), "{}").unwrap();
        std::fs::write(data.join("saves/notes.json"), "{}").unwrap();
        std::fs::write(data.join("saves/4.json"), "cut sh").unwrap();
        assert_eq!(new_path(&data), data.join("saves/5.json"));
        let paths: Vec<PathBuf> = list(&data).into_iter().map(|s| s.path).collect();
        assert_eq!(paths, vec![first.clone(), second.clone()]);
        delete(&second).unwrap();
        assert!(!second.exists() && !data.join("saves/2.old.json").exists());
        assert!(data.join("saves/3.old.json").exists());
        let _ = std::fs::remove_dir_all(&data);
    }
}
