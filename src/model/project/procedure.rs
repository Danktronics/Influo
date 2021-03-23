use std::fmt;

use anyhow::{Error, anyhow};
use serde_json::Value;
use serde::{Serialize, Deserialize, Deserializer, de};
use serde::de::{Visitor, MapAccess};

#[derive(Debug, Serialize, Deserialize)]
pub struct Procedure {
    pub name: String,
    pub commands: Vec<String>,
    pub environment: String,
    pub condition: String,
    pub deploy_path: Option<String>,
    pub auto_restart: AutoRestartPolicy,
    pub branches: Vec<String>,
    pub log: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum AutoRestartPolicy {
    Always, // If the command was unsuccessful, restart
    Never, // If the command was unsuccessful, don't restart
    ExclusionCodes(Vec<i32>), // If the command was unsuccessful and if it is NOT one of the exclusion codes restart
    InclusionCodes(Vec<i32>), // If the command was unsuccessful and if it is one of the inclusion codes restart
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
                while let Ok(Some((key, value))) = map.next_entry::<&str, Vec<i32>>() {
                    if key == "only" {
                        only = Some(value);
                    } else if key == "not" {
                        not = Some(value);
                    }
                }

                if only.is_some() && not.is_some() {
                    return Err(de::Error::custom(format!("Found both \"only\" and \"not\" in auto_restart")));
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
