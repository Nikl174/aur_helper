use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::process::Command;
use std::process::ExitStatus;

// TODO:
// aur_helper update [dir]
// aur_helper build [dir]
// ? aur_helper install [dir]
fn main() {
    let mut args = env::args();
    // skip programm name
    args.next();
    let dir = args.next();

    let path = match dir {
        Some(x) => x,
        None => {
            println!("No argument provided!");
            return;
        }
    };

    let mut dirs: Vec<PathBuf> = Vec::new();
    match get_dirs(Path::new(&path), true) {
        Ok(x) => {
            for i in &x {
                let p = i.to_str().unwrap();
                println!("{p}");
            }
            dirs = x;
        }
        Err(x) => println!("Error: {}", x),
    }
    match update_packages(&dirs) {
        Ok(vec) => {
            for x in vec {
                println!("Updated package: {}", x.to_str().unwrap());
            }
        }
        Err(vec) => {
            for x in vec {
                println!(
                    "Couldn't update package {}, failed with error {}",
                    x.0.to_str().unwrap(),
                    x.1.to_string()
                )
            }
        }
    }
}

// goes through the directories and calls 'git pull' and returns a vector of successfull updated
// dirs the exit status, if a command fails
fn update_packages(dirs: &Vec<PathBuf>) -> Result<Vec<PathBuf>, Vec<(PathBuf, ExitStatus)>> {
    let mut failed_dirs: Vec<(PathBuf, ExitStatus)> = Vec::new();
    let mut success_dirs: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let status = Command::new("git")
            .arg("pull")
            .current_dir(dir.clone())
            .status()
            .expect("Failed to execute git pull!");
        if status.success() {
            success_dirs.push(dir.clone());
        } else {
            failed_dirs.push((dir.clone(), status));
        }
    }
    if failed_dirs.is_empty() {
        return Ok(success_dirs);
    }
    Err(failed_dirs)
}

// returns the directories in path and warns if it's a wrong directory
fn get_dirs(current_path: &Path, warn_wrong_dir: bool) -> Result<Vec<PathBuf>, io::Error> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if !current_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Given Path is not a directory!",
        ));
    }

    for path in fs::read_dir(current_path).expect("Couldn't read path!") {
        let dir_ent = path.expect("Dir-entry error!").path();

        if warn_wrong_dir && !dir_ent.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "AUR Path contains not only directorys, right AUR-dir?",
            ));
        }

        paths.push(dir_ent);
    }
    Ok(paths)
}
#[cfg(test)]
mod get_dirs {

    use super::*;

    // Helper function for a test preperation, creates a temp directory for test purposes
    fn prepair_tmp_dir(tmp_dir_abs_path: &str) {
        clean_up_tmp_dir(tmp_dir_abs_path);
        Command::new("mkdir")
            .arg("-p")
            .arg(tmp_dir_abs_path)
            .status()
            .expect("mkdir couldn't execute to create temporary test dir");
    }

    // Helper function for clean up the created test directory after test
    fn clean_up_tmp_dir(tmp_dir_abs_path: &str) {
        if Command::new("ls")
            .arg(tmp_dir_abs_path)
            .status()
            .expect("Couldn't remove test directory with rm")
            .success()
        {
            Command::new("rm")
                .arg("-r")
                .arg(tmp_dir_abs_path)
                .status()
                .expect("Couldn't remove test directory with rm");
        }
    }

    #[test]
    fn get_dirs_detects_right_test() {
        // prepair
        let tmp_path = "/tmp/aur_helper_rs_test/get_dirs_detects_right_test".to_string();
        prepair_tmp_dir(&tmp_path.as_str());
        Command::new("mkdir")
            .current_dir(tmp_path.as_str())
            .arg("test_dir1")
            .arg("test_dir2")
            .arg("test_dir3")
            .status()
            .expect("Couldn't create subdirs in test");

        // TEST
        let paths = get_dirs(Path::new("/tmp/aur_helper_rs_test/"), true);

        assert!(paths.is_ok());

        let mut actual_paths: Vec<PathBuf> = Vec::new();
        actual_paths.push(Path::new(&(tmp_path.clone() + "test_dir1")).to_path_buf());
        actual_paths.push(Path::new(&(tmp_path.clone() + "test_dir2")).to_path_buf());
        actual_paths.push(Path::new(&(tmp_path.clone() + "test_dir3")).to_path_buf());

        assert_eq!(paths.unwrap().sort(), actual_paths.sort());

        // clean up
        clean_up_tmp_dir(&tmp_path);
    }

    #[test]
    fn get_dirs_error_test() {
        // prepair
        let tmp_path = "/tmp/aur_helper_rs_test/get_dirs_error_test";
        let test_file = tmp_path.to_owned() + "/test_file.txt";
        prepair_tmp_dir(&tmp_path);
        println!("{}", test_file.as_str());

        Command::new("touch")
            .arg(test_file.as_str())
            .status()
            .expect("Failed to touch a file in the test directory");

        // TEST
        let err = get_dirs(Path::new(tmp_path), true);
        let err2 = get_dirs(Path::new(test_file.as_str()), true);

        assert!(err.is_err());
        assert!(err2.is_err());

        // clean up
        clean_up_tmp_dir(&tmp_path);
    }
}
