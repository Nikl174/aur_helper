use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

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
    println!("Path: {path}");

    match get_dirs(Path::new(&path)) {
        Ok(x) => {
            for i in &x {
                let p = i.to_str().unwrap();
                println!("{p}");
            }
        }
        Err(x) => println!("Error: {}", x),
    }
}

fn get_dirs(current_path: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if current_path.is_dir() {
        for path in fs::read_dir(current_path).expect("Couldn't read path!") {
            let dir_ent = path.expect("Dir-entry error!").path();
            if dir_ent.is_dir() {
                paths.push(dir_ent);
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "AUR Path contains not only directorys, right AUR-dir?",
                ));
            }
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Given Path is not a directory!",
        ));
    }
    Ok(paths)
}
