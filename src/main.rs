fn main() {
    print_supported_mcus();
}



fn print_supported_mcus() {
    println!("Supported MCUs are:");
    for name in supported_mcus() {
        println!(" - {}", name);
    }
}
