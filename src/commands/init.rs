use crate::cli::InitArgs;
use crate::error::{GraftError, Result};
use std::path::Path;

const TOML_TEMPLATE: &str = r#"# Graft configuration
# See: https://github.com/user/graft for documentation

# [managers]
# macos = "brew install"
# arch = "pacman -S --noconfirm"
# ubuntu = "sudo apt install -y"

# [neovim]
# os = ["macos", "linux"]
# install = "neovim"
# files = { "nvim/" = "~/.config/nvim" }
# tags = ["editor"]
"#;

const YAML_TEMPLATE: &str = r#"# Graft configuration
# See: https://github.com/user/graft for documentation

# managers:
#   macos: "brew install"
#   arch: "pacman -S --noconfirm"
#   ubuntu: "sudo apt install -y"

# neovim:
#   os: [macos, linux]
#   install: neovim
#   files:
#     "nvim/": "~/.config/nvim"
#   tags: [editor]
"#;

const JSON_TEMPLATE: &str = r#"{
  "_comment": "Graft configuration — See: https://github.com/user/graft for documentation",
  "managers": {},
  "packages": {}
}
"#;

const CONFIG_FILES: &[&str] = &["graft.toml", "graft.yaml", "graft.json"];

pub fn run(args: &InitArgs) -> Result<()> {
    // Check if any config file already exists
    for name in CONFIG_FILES {
        let path = Path::new(name);
        if path.exists() {
            return Err(GraftError::ConfigAlreadyExists(name.to_string()));
        }
    }

    let format = args.format.as_deref().unwrap_or("toml");
    let (filename, content) = match format {
        "toml" => ("graft.toml", TOML_TEMPLATE),
        "yaml" => ("graft.yaml", YAML_TEMPLATE),
        "json" => ("graft.json", JSON_TEMPLATE),
        _ => return Err(GraftError::ConfigParse(format!("unsupported format: {format}"))),
    };

    std::fs::write(filename, content)
        .map_err(|e| GraftError::ConfigParse(format!("failed to write {filename}: {e}")))?;

    println!("Created {filename}");
    Ok(())
}
