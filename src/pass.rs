extern crate glob;
use self::glob::glob;

extern crate notify;
use self::notify::{RecommendedWatcher, Watcher, RecursiveMode};
use self::notify::DebouncedEvent::Create;
use std::sync::mpsc::{Sender, channel, SendError};
use std::time::Duration;
use std::error::Error;
use std::path::{PathBuf, Path};
use std::env;

#[derive(Clone)]
pub struct Password {
    pub name: String,
    pub meta: String,
    pub filename: String,
}

pub fn load_and_watch_passwords(tx: Sender<Password>) -> Result<(), Box<Error>> {
    try!(load_passwords(&tx));
    try!(watch_passwords(tx));
    Ok(())
}

fn to_name(path: &PathBuf) -> String {
    path.file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
        .trim_right_matches(".gpg")
        .to_string()
}

fn to_password(path: PathBuf) -> Password {
    Password {
        name: to_name(&path),
        filename: path.to_string_lossy().into_owned().clone(),
        meta: "".to_string(),
    }
}

/// Determine password base path
fn  password_base() -> Option<PathBuf>{
    let mut pass_home = match env::var("PASSWORD_STORE_DIR"){
        Ok(p) => {p}
        Err(_) => {"".into()}
    };
    let homedir_path;
    if !Path::new(&pass_home).exists(){
        homedir_path = env::home_dir().unwrap().join(".password-store");
        pass_home = homedir_path.to_string_lossy().into();
    }
    return Some(Path::new(&pass_home).to_path_buf());
}

fn load_passwords(tx: &Sender<Password>) -> Result<(), SendError<Password>> {
    let password_path = password_base().unwrap();
    let password_path_glob = password_path.join("**/*.gpg");

    // Find all passwords
    let ref passpath_str = password_path_glob.to_string_lossy();
    println!("path: {}", passpath_str);
    for entry in glob(passpath_str).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => try!(tx.send(to_password(path))),
            Err(e) => println!("{:?}", e),
        }
    }
    Ok(())
}

fn watch_passwords(password_tx: Sender<Password>) -> Result<(), Box<Error>> {
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = try!(Watcher::new(tx, Duration::from_secs(2)));
    let pass_base = password_base().unwrap();
    try!(watcher.watch(pass_base, RecursiveMode::Recursive));

    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    Create(path) => try!(password_tx.send(to_password(path))),
                    _ => (),
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}