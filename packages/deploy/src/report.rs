use serde::Serialize;

#[derive(Serialize)]
pub struct DeploymentReport {
    pub chain_id: String,
    pub contracts: Vec<(String, String, u64)>,
    pub kernel_address: String,
}

impl DeploymentReport {
    pub fn write_to_json(&self) -> Result<(), std::io::Error> {
        let reports_dir = "./deployment-reports";
        if !std::path::Path::new(reports_dir).exists() {
            std::fs::create_dir_all(reports_dir)?;
        }
        let file_name = format!("./deployment-reports/{}_deployment.json", self.chain_id);
        let json_data = serde_json::to_string_pretty(&self)?;
        std::fs::write(file_name, json_data)?;
        Ok(())
    }
}
