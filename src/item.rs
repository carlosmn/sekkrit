use std::fmt;

use serde::de::{self, Deserialize, Deserializer, Visitor, MapAccess};
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

#[derive(Debug)]
pub enum LoginField {
    Text{value: String, name: String, designation: Option<String>},
    Password{value: String, name: String, designation: Option<String>},
    Info{value: String, name: String, designation: Option<String>},
    Checkbox{value: String, name: String, designation: Option<String>},
    B{value: String, name: String, designation: Option<String>},
}

impl<'de> Deserialize<'de> for LoginField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        enum Field { Type, Value, Name, Designation };

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: Deserializer<'de>
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a login field attribute")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where E: de::Error
                    {
                        match value {
                            "type" => Ok(Field::Type),
                            "value" => Ok(Field::Value),
                            "name" => Ok(Field::Name),
                            "designation" => Ok(Field::Designation),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct LoginFieldVisitor;

        impl<'de> Visitor<'de> for LoginFieldVisitor {
            type Value = LoginField;


            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a login field")
            }

            fn visit_map<V>(self, mut map: V) -> Result<LoginField, V::Error>
                where V: MapAccess<'de>
            {
                let mut type_ = None;
                let mut value = None;
                let mut name = None;
                let mut designation = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Type => {
                            if type_.is_some() {
                                return Err(de::Error::duplicate_field("type"));
                            }
                            type_ = Some(map.next_value()?);
                        },
                        Field::Value => {
                            if value.is_some() {
                                return Err(de::Error::duplicate_field("value"));
                            }
                            value = Some(map.next_value()?);
                        },
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        },
                        Field::Designation => {
                            if designation.is_some() {
                                return Err(de::Error::duplicate_field("designation"));
                            }
                            designation = Some(map.next_value()?);
                        },
                    }
                }

                struct ExpectedType;

                impl de::Expected for ExpectedType {
                    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("T or P as type")
                    }
                }

                let type_ = type_.ok_or_else(|| de::Error::missing_field("type"))?;
                let value = value.ok_or_else(|| de::Error::missing_field("value"))?;
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;

                match type_ {
                    "T" => Ok(LoginField::Text{name: name, value: value, designation: designation}),
                    "P" => Ok(LoginField::Password{name: name, value: value, designation: designation}),
                    "I" => Ok(LoginField::Info{name: name, value: value, designation: designation}),
                    "C" => Ok(LoginField::Checkbox{name: name, value: value, designation: designation}),
                    "B" => Ok(LoginField::B{name: name, value: value, designation: designation}),
                    _ => return Err(de::Error::invalid_value(de::Unexpected::Str(type_), &ExpectedType)),
                }
            }
        }

        const FIELDS: &'static [&'static str] = &["type", "name", "value", "designation"];
        deserializer.deserialize_struct("LoginField", FIELDS, LoginFieldVisitor)
    }
}

impl Login {
    pub fn from_slice(s: &[u8]) -> Result<Login, serde_json::Error> {
        serde_json::from_slice(s)
    }
}
