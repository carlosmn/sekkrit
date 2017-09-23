use serde_json;

#[derive(Deserialize)]
pub struct Section {
    name: String,
    title: String,
    sections: Vec<SectionField>
}

pub enum SectionKind {
    String,
    Concealed,
}

pub struct SectionField {
    k: String,
    n: String,
    v: String,
    t: String,
}
