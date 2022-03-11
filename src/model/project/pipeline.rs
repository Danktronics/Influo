use std::collections::HashMap;
use std::convert::TryFrom;
use serde::{Serialize, Deserialize, Deserializer};
use serde::de::{Visitor, MapAccess, SeqAccess};

use super::procedure::Procedure;

use std::fmt;
use std::marker;
use serde::de;

use anyhow::anyhow;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pipeline {
    pub name: String,
    pub stages_order: Option<Vec<String>>,
    pub branches: Vec<String>,
    pub deploy_path: Option<String>,
    pub log: Option<Log>,
    pub condition: Condition,
    pub stages: HashMap<String, Stage>,
    #[serde(default)]
    pub persistent: bool
}

impl TryFrom<IntermediatePipeline> for Pipeline {
    type Error = String;

    fn try_from(intermediate: IntermediatePipeline) -> Result<Self, Self::Error> {
        let stages_order;
        match intermediate.stages_order {
            Some(order) => {
                for stage_name in &order {
                    if !intermediate.stages.hash_map.contains_key(stage_name) {
                        return Err(format!("Stage with name: {} mentioned in order does not exist", &stage_name));
                    }
                }

                stages_order = Some(order);
            },
            None => {
                stages_order = Some(intermediate.stages.order);
            }
        };

        Ok(Pipeline {
            name: intermediate.name,
            stages_order: stages_order,
            branches: intermediate.branches,
            deploy_path: intermediate.deploy_path,
            log: intermediate.log,
            condition: intermediate.condition,
            stages: intermediate.stages.hash_map,
            persistent: intermediate.persistent
        })
    }
}

/// Represents raw data from configuration
#[derive(Debug, Deserialize)]
pub struct IntermediatePipeline {
    pub name: String,
    pub stages_order: Option<Vec<String>>,
    pub branches: Vec<String>,
    pub deploy_path: Option<String>,
    pub log: Option<Log>,
    pub condition: Condition,
    pub stages: HashMapWithOriginalOrder<String, Stage>,
    #[serde(default)]
    pub persistent: bool
}

#[derive(Debug)]
pub struct HashMapWithOriginalOrder<K, V> {
    hash_map: HashMap<K, V>,
    order: Vec<K>
}

impl<'de, K, V> Deserialize<'de> for HashMapWithOriginalOrder<K, V>
where
    K: std::fmt::Debug + Deserialize<'de> + Eq + std::hash::Hash + Clone,
    V: Deserialize<'de>
{
    fn deserialize<D>(deserializer: D) -> Result<HashMapWithOriginalOrder<K, V>, D::Error>
    where
        D: Deserializer<'de>
    {
        struct StagesObjectVisitor<K, V> {
            phantom_marker: marker::PhantomData<HashMapWithOriginalOrder<K, V>>
        }
    
        impl<'de, K, V> Visitor<'de> for StagesObjectVisitor<K, V>
        where
            K: std::fmt::Debug + Deserialize<'de> + Eq + std::hash::Hash + Clone,
            V: Deserialize<'de>
        {
            type Value = HashMapWithOriginalOrder<K, V>;
            
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an object containing stage objects")
            }
    
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>
            {
                let mut hash_map: HashMap<K, V> = HashMap::new();
                let mut order: Vec<K> = Vec::new();

                while let Ok(Some((key, value))) = map.next_entry::<K, V>() {
                    hash_map.insert(key.clone(), value);
                    order.push(key);
                }

                Ok(Self::Value {
                    hash_map: hash_map,
                    order: order
                })
            }
        }
    
        deserializer.deserialize_map(StagesObjectVisitor {
            phantom_marker: marker::PhantomData
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Log {
    pub template: Option<String>,
    pub console: Option<bool>,
    pub save_to_file: Option<bool>,
    pub file_path: Option<String>,
    pub in_memory: Option<bool>
}

impl Log {
    pub fn is_enabled(&self) -> bool {
        self.console.unwrap_or(false) || self.save_to_file.unwrap_or(false) || self.in_memory.unwrap_or(false)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Condition {
    Automatic,
    Manual
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Stage {
    Multiple(Vec<Procedure>),
    Single(Procedure)
}

/*impl<'de> Deserialize<'de> for Stage {
    fn deserialize<D>(deserializer: D) -> Result<Stage, D::Error>
    where
        D: Deserializer<'de>
    {
        struct StageVisitor;
    
        impl<'de> Visitor<'de> for StageVisitor {
            type Value = Stage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a stage object or array of stage objects")       
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>
            {
                let procedures: Vec<Procedure> = Vec::new();

                while let Ok(Some(element)) = seq.next_element() {
                    procedures.push(element);
                }

                Ok(Stage::Multiple(procedures))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>
            {
                
            }
        }

        deserializer.deserialize_any(StageVisitor)
    }
}*/
