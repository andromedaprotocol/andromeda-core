use std::collections::HashMap;

use cosmwasm_std::Addr;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Config {
    pub installations: HashMap<String, Addr>,
}

impl Config {
    pub fn load() -> Self {
        let contents = std::fs::read_to_string("config.yml").expect("Unable to read file");
        let config: Self = serde_yml::from_str(&contents).expect("Unable to parse YAML");
        config
    }

    pub fn save(&self) {
        let contents = serde_yml::to_string(self).expect("Unable to parse YAML");
        std::fs::write("config.yml", contents).expect("Unable to write file");
    }

    pub fn get_installation(&self, name: &str) -> &Addr {
        self.installations
            .get(name)
            .unwrap_or_else(|| panic!("Installation not found: {}", name))
    }
}
