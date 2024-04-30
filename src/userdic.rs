use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn read_user_dict(p:&Path) -> HashMap<char, Vec<char>> {
    let mut table = HashMap::new();

    if let Ok(file) = File::open(p.join("user.dict")) {
        let reader = BufReader::new(file);

        for line in reader.lines() {
            if let Ok(entry) = line {
                let parts: Vec<char> = entry.trim().chars().collect();
                if parts.len() == 3 {
                    table.entry(parts[0])
                        .or_insert_with(Vec::new)
                        .push(parts[2]);
                }
            }
        }
    }
    table
}
