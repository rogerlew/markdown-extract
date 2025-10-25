use std::process;

fn main() {
    match markdown_doc_cli::run() {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("markdown-doc error: {err}");
            process::exit(1);
        }
    }
}
