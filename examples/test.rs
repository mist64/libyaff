use libyaff::{YaffFont, to_yaff_string};

fn main() {
    let path_in = std::env::args()
        .nth(1)
        .expect("Please provide an input file path as the first argument");
    let path_out = std::env::args()
        .nth(2)
        .expect("Please provide an output file path as the second argument");
    let path_in2 = path_in.clone();
    match YaffFont::from_path(path_in) {
        Ok(font) => {
            let yaff_string = to_yaff_string(&font);
            std::fs::write(path_out, yaff_string).expect("Unable to write file");
        }
        Err(e) => {
            eprintln!("Failed to parse YAFF data from file {}: {:?}", path_in2, e);
            std::process::exit(1);
        }
    }
}
