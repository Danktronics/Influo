use std::fmt;

use serde::{Serializer, Serialize, Deserialize, Deserializer, de};
use serde::ser::SerializeStruct;
use serde::de::{Visitor, MapAccess};

use super::pipeline::Condition;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Procedure {
    pub name: Option<String>,
    pub commands: Vec<String>,
    pub condition: Condition,
    pub auto_restart: AutoRestartPolicy,
    pub log_template: Option<String>,
    #[serde(default)]
    pub persistent: bool
}

#[derive(Debug, Clone)]
pub enum AutoRestartPolicy {
    Always, // If the command was unsuccessful, restart
    Never, // If the command was unsuccessful, don't restart
    ExclusionCodes(Vec<i32>), // If the command was unsuccessful and if it is NOT one of the exclusion codes restart
    InclusionCodes(Vec<i32>), // If the command was unsuccessful and if it is one of the inclusion codes restart
}

impl Serialize for AutoRestartPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        match self {
            AutoRestartPolicy::Always => serializer.serialize_bool(true),
            AutoRestartPolicy::Never => serializer.serialize_bool(false),
            AutoRestartPolicy::ExclusionCodes(exclusion_codes) => {
                let mut state = serializer.serialize_struct("AutoRestartPolicy", 1)?;
                state.serialize_field("not", &exclusion_codes)?;
                state.end()
            },
            AutoRestartPolicy::InclusionCodes(inclusion_codes) => {
                let mut state = serializer.serialize_struct("AutoRestartPolicy", 1)?;
                state.serialize_field("only", &inclusion_codes)?;
                state.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for AutoRestartPolicy {
    fn deserialize<D>(deserializer: D) -> Result<AutoRestartPolicy, D::Error>
    where
        D: Deserializer<'de>
    {
        struct AutoRestartPolicyVisitor;

        impl<'de> Visitor<'de> for AutoRestartPolicyVisitor {
            type Value = AutoRestartPolicy;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a boolean or an object containing \"only\" or \"not\"")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error
            {
                Ok(if v {
                    AutoRestartPolicy::Always
                } else {
                    AutoRestartPolicy::Never
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>
            {
                let mut only = None;
                let mut not = None;
                while let Ok(Some((key, value))) = map.next_entry::<String, Vec<i32>>() {
                    if key == "only" {
                        only = Some(value);
                    } else if key == "not" {
                        not = Some(value);
                    }
                }

                if only.is_some() && not.is_some() {
                    return Err(de::Error::custom("Found both \"only\" and \"not\" in auto_restart"));
                }

                if let Some(only) = only {
                    return Ok(AutoRestartPolicy::InclusionCodes(only));
                } else if let Some(not) = not {
                    return Ok(AutoRestartPolicy::ExclusionCodes(not));
                }

                Err(de::Error::custom("Missing auto_restart condition (\"only\" or \"not\")"))
            }
        }

        deserializer.deserialize_any(AutoRestartPolicyVisitor)
    }
}
