fn main() {
    println!("{}", serde_json::to_string_pretty(&fsc_commands::catalogue()).expect("catalogue serializes"));
}
