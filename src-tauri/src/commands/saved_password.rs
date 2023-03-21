#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct SavedPassword {
    pub pw: String,
    pub name: String,
}
