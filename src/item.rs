use serde_json;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    html_form: Option<HtmlForm>,
    fields: Vec<LoginField>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HtmlForm {
    html_id: Option<String>,
    html_name: Option<String>,
    html_method: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginField {
    #[serde(rename = "type")]
    type_: String,
    value: String,
    designation: String,
    name: String,
}

impl Login {
    pub fn from_slice(s: &[u8]) -> Result<Login, serde_json::Error> {
        serde_json::from_slice(s)
    }
}
