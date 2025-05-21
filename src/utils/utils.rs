use std::{
    fs::{OpenOptions, create_dir_all, read_dir, read_to_string},
    io::{Read, Write},
    path::Path,
    process::Command,
};

pub fn get_git_infos() -> () {}

pub fn execute_commande(commande: &str) -> Result<String, String> {
    match Command::new("sh").arg("-c").arg(commande).output() {
        Ok(output) => {
            if !output.status.success() {
                // let y = String::
                return Err(String::from_utf8_lossy(&output.stderr).to_string());
            }
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        Err(err) => Err(err.to_string()),
    }
}

pub fn check_dir_exist_or_create(file_path: &str) -> () {
    let tmp_path = format!("{}", file_path);
    // Convert the file path to a Path
    let path = Path::new::<String>(&tmp_path);

    // Create all directories in the path if they don't exist
    if let Some(parent) = path.parent() {
        if let Err(err) = create_dir_all(parent) {
            println!("Failed to create directories: {}", err);
        }
    }
}

pub fn write_to_file_ut(file_path: &str, content: &str) -> Result<bool, String> {
    // Create all directories in the path if they don't exist
    check_dir_exist_or_create(&file_path);

    // Open or create the file
    let mut file = match OpenOptions::new().create(true).append(true).open(file_path) {
        Ok(f) => f,
        Err(err) => return Err(format!("Failed to open file: {}", err)),
    };

    // Write the content
    match file.write_all(content.as_bytes()) {
        Ok(_) => Ok(true),
        Err(err) => Err(format!("Failed to write to file: {}", err)),
    }
}
pub fn read_from_file_ut(file_path: &str) -> Result<String, String> {
    match read_to_string(file_path) {
        Ok(f) => return Ok(f),
        Err(err) => {
            return Err(err.to_string());
        }
    };
}

pub fn list_dir_contents(path: &str) -> Result<Vec<String>, bool> {
    let dir_content = match read_dir(path) {
        Ok(content) => content,
        Err(err) => {
            print!("{}", err.to_string());
            return Err(false);
        }
    };

    let mut content = Vec::<String>::new();

    for entry in dir_content {
        let curr_entry = match entry {
            Ok(curr_entry) => curr_entry,
            Err(err) => {
                print!("{}", err.to_string());
                continue;
            }
        };

        let path = curr_entry.path();

        if path.is_dir() {
            continue;
        }

        let y = match path.file_name() {
            Some(f_name) => f_name,
            None => {
                continue;
            }
        };

        let mut str_file = String::new();

        let _ = y.as_encoded_bytes().read_to_string(&mut str_file);

        content.push(str_file);
    }
    return Ok(content);
}

pub fn extract_repo_info(url: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = url.split('/').collect();

    // We need at least username and repo parts
    if parts.len() < 2 {
        return None;
    }

    // Get the last part (repo) and strip .git suffix
    let repo = parts.last()?.strip_suffix(".git")?;

    // The username should be second-to-last for standard GitHub URLs
    // Handle cases like "https://github.com/owner/repo.git"
    let username = parts.get(parts.len() - 2)?;

    Some((username, repo))
}
