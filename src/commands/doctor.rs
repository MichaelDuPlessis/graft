use crate::error::Result;
use crate::platform;
use colored::Colorize;

pub fn run() -> Result<()> {
    match platform::detect(None) {
        Ok(p) => println!("{} Detected platform: {}", "✓".green(), p.0.bold()),
        Err(e) => println!("{} {}", "✗".red(), e),
    }
    Ok(())
}
