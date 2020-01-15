extern crate mime_guess;
extern crate s3;

use s3::bucket::Bucket;
use s3::credentials::Credentials;
use s3::error::S3Error;

const BUCKET: &str = "ddd-3";
const REGION: &str = "eu-central-1";
const CREDENTIALS_PROFILE: &str = "rust-s3";

use notify::{
    event::CreateKind, event::ModifyKind, Event, EventKind, RecommendedWatcher, RecursiveMode, Result, Watcher,
};
use std::{fs::File, io::Read, path::PathBuf, result, thread, time::Duration};

// Checks if a file already exists in S3
fn file_exists(bucket: &Bucket, file_name: &str) -> bool {
    let results = bucket.list_all("".to_string(), None).unwrap();

    for result in results {
        let result_object = &result.0;
        let list = &result_object.contents;

        let exists = list.iter().any(|item| item.key == file_name);

        if exists {
            return true;
        }
    }

    false
}

// Guess the MIME type based on the filename
fn get_mime_type(file_name: &str) -> String {
    let guess = mime_guess::from_path(file_name);
    let mime = guess.first_or_text_plain();
    let type_ = mime.type_().as_str();
    let subtype = mime.subtype().as_str();
    String::from(type_) + "/" + &subtype
}

fn upload(bucket: &Bucket, file_path: &PathBuf) -> result::Result<(), S3Error> {
    println!("Uploading file... {:?}", file_path);

    let file_path_string = file_path.to_str().unwrap();
    let file_name = file_path.file_name().unwrap();
    let file_name_string = file_name.to_str().unwrap();

    if file_exists(&bucket, file_name_string) {
        println!("File already exists in S3. Aborting...");

        let error = S3Error {
            description: Some(String::from("File already exists.")),
            data: None,
        };

        return Err(error);
    }

    let mut file = File::open(file_path_string).expect("Source file doesn’t exist");
    let mut content: Vec<u8> = vec![];

    file.read_to_end(&mut content).unwrap();

    let mime_type = get_mime_type(&file_name_string);

    println!("mime {:?}", mime_type);

    let upload_result = bucket.put_object(file_name_string, content.as_ref(), &mime_type);

    match upload_result {
        Ok((_, code)) => {
            if code == 200 {
                println!("File successfully uploaded!");
            }
        }
        Err(e) => println!("Error uploading file! {:?}", e),
    }

    Ok(())
}

fn watch() -> Result<()> {
    let region = REGION.parse().unwrap();

    let credentials = Credentials::default();

    let bucket = Bucket::new(BUCKET, region, credentials)
        .expect("Could not initialize bucket. Used the right credentials?");

    let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res: Result<Event>| {
        match res {
            Ok(event) => {
                let file_path = event.paths.get(0).unwrap();

                match event.kind {
                    EventKind::Create(_s) => upload(&bucket, &file_path),
                    EventKind::Modify(s) => {
                        println!("kind: {:?}", s);

                        match s {
                            ModifyKind::Name(f) => println!("hoi {:?}", f),
                            _ => {}
                        }

                        upload(&bucket, &file_path)
                    },
                    _ => Ok(()),
                }
            }
            Err(err) => {
                Ok(())
            },
        };
    })?;

    watcher.watch("./videos", RecursiveMode::NonRecursive)?;

    thread::sleep(Duration::from_secs(100000));

    Ok(())
}

fn main() {
    watch().unwrap();
}
