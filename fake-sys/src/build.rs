use std::os;
use std::io::{File,Command};

fn main() {
    let out_dir = os::getenv("OUT_DIR").unwrap();
    let dst = Path::new(out_dir.clone());

    let mut process = match Command::new("./build.sh").spawn() {
      Ok(p) => p,
      Err(e) => panic!("failed to execute process: {}", e),
    };
    let output = process.stdout.as_mut().unwrap().read_to_string()
        .ok().expect("failed to read libfake build output");
    let mut f = File::create(&dst.join("gen_fake_log.txt")).unwrap();
    f.write_str(output.as_slice()).unwrap();
    println!("cargo:rustc-flags=-L {} -l fake:static", out_dir);
}
