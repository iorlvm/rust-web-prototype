pub struct Config {
    addr: String,
    port: u16,
}

impl Config {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.addr, self.port)
    }
}

// 負責檔案載入, 以及提供預設設定值
pub async fn load_config() -> Config {
    // TODO: 後續實現
    Config {
        addr: "0.0.0.0".to_string(),
        port: 8080,
    }
}
