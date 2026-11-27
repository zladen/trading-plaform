fn main() {
    println!(
        "Минимальная версия Rust: {}",
        env!("CARGO_PKG_RUST_VERSION")
    );
    println!("Программа собрана Cargo");
}
