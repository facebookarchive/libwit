use std::os;
use std::io::{File,Command};

fn main() {
    let out_dir = os::getenv("OUT_DIR").unwrap();
    let dst = Path::new(out_dir.clone());

    let mut process = match Command::new("./gen_vad.sh").spawn() {
      Ok(p) => p,
      Err(e) => panic!("failed to execute process: {}", e),
    };
    let output = process.stdout.as_mut().unwrap().read_to_string()
        .ok().expect("failed to read vad build output");
    let mut f = File::create(&dst.join("gen_vad_log.txt")).unwrap();
    f.write_str(output.as_slice()).unwrap();
    println!("cargo:rustc-flags=-L {} -l vad:static", out_dir);
}
