use tabled::Tabled;

#[derive(Tabled)]
pub struct TableData {
    pub key: String,
    pub value: String,
}
