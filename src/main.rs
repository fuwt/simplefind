use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};

use regex::bytes::Regex;

mod userdic;
use userdic::read_user_dict;
use dirs;
use std::env;



fn table_match(needle: Vec<char>, haystack: Vec<char>, dict:MutexGuard<'_, HashMap<char, Vec<char>>>) -> bool {
    let needle_len = needle.len();
    let haystack_len = haystack.len();
    if needle_len > haystack_len { return false }

    for i in 0..=(haystack_len - needle_len) {
        let mut found = true;
        for j in 0..needle_len {
            if haystack[i + j] == needle[j] { continue; }
            if let Some(value) = dict.get(&haystack[i+j]) {
                if !value.contains(&needle[j]) {
                    found = false;
                    break;
                }
            } else {
                found = false;
                break;
            }
        }
        if found {
            return true
        }
    }
    false
}

fn matches(needle:&String, name: &OsStr, dict:MutexGuard<'_, HashMap<char, Vec<char>>>) -> bool{
    let regex = Regex::new(needle).unwrap();
    let _ret = regex.is_match(name.as_encoded_bytes());
    if _ret { return  regex.is_match(name.as_encoded_bytes())}
    else {
        let name_bytes = name.as_encoded_bytes();
        return match std::str::from_utf8(name_bytes) {
            Ok(s) => return table_match(needle.chars().collect(), s.chars().collect(), dict),
            Err(_) => false
        }
    }
}


fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn traverse_directory(dir: &Path, query: &String, dict:HashMap<char, Vec<char>> ) -> Vec<PathBuf> {
    let dirs = Arc::new(Mutex::new(vec![dir.to_path_buf()]));
    let result = Arc::new(Mutex::new(Vec::new()));
    let active_threads = Arc::new(AtomicUsize::new(0));
    let dict = Arc::new(Mutex::new(dict));

    let mut handles = vec![];
    for _i in 0..4 {
        let dirs = Arc::clone(&dirs);
        let result = Arc::clone(&result);
        let active_threads = Arc::clone(&active_threads);
        let dict = Arc::clone(&dict);
        let query_copy = query.clone();
        let handle = thread::spawn(move || {
            loop {
                let curdir;
                { curdir = dirs.lock().unwrap().pop(); }
                match curdir {
                    Some(d) => {
                        active_threads.fetch_add(1, Ordering::Relaxed);
                        if let Ok(entries) = fs::read_dir(d) {
                            let mut local_result = vec![];
                            let mut local_dirs = vec![];
                            for entry in entries {
                                if let Ok(entry) = entry {
                                    let path = entry.path();
                                    if is_hidden(&path) {
                                        continue;
                                    }
                                    if path.is_dir() {
                                        local_dirs.push(path.clone());
                                    }
                                    let name = path.file_name().unwrap();
                                    let dict = dict.lock().unwrap();
                                    if matches(&query_copy, &name, dict){
                                        // println!("{}", path.display());
                                        local_result.push(path);
                                    }
                                }
                            }
                            result.lock().unwrap().extend(local_result);
                            dirs.lock().unwrap().extend(local_dirs);
                        }
                        active_threads.fetch_sub(1, Ordering::Relaxed);
                    },
                    None => {
                        if active_threads.load(Ordering::Relaxed) == 0 { break }
                    }
                }
            }
        });
        handles.push(handle);
    }
    for handle in handles  {
        handle.join().expect("Thread panicked");
    }
    Arc::try_unwrap(result).unwrap().into_inner().unwrap()
}


fn main() {
    let dictpath = env::var_os("XDG_CONFIG_HOME")
				.map(PathBuf::from)
				.filter(|p| p.is_absolute())
				.or_else(|| dirs::home_dir().map(|h| h.join(".config")))
				.expect("Failed to get config directory");
    // let dictpath = Path::new("/home/fwt/Sync/dotfile/config/yazi");
    let dict = read_user_dict(&dictpath);
    let args: Vec<String> = std::env::args().collect();
    let query = &args[1];
    let start_time = Instant::now();
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let _paths = traverse_directory(&current_dir, &query, dict);
    for p in _paths {
        println!("{}", p.display());
    }
    let end_time = Instant::now();
    let duration = (end_time - start_time)/10;
    println!("程序运行时间: {:.2?}", duration);
}
