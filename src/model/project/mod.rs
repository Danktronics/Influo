use serde::{Serialize, Deserialize, Deserializer};

pub mod pipeline;
pub mod procedure;
pub mod branch;

use self::{
    pipeline::Pipeline,
    branch::Branch
};

use self::pipeline::IntermediatePipeline;

use std::convert::TryFrom;

use serde::de;

use serde::de::Unexpected;

use serde::de::Visitor;

use std::fmt;

use serde::de::SeqAccess;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub url: String,
    #[serde(deserialize_with = "deserialize_pipeline")]
    pub pipelines: Vec<Pipeline>,
    #[serde(skip)]
    pub branches: Vec<Branch>,
    #[serde(default)]
    pub persistent: bool
}

fn deserialize_pipeline<'de, D>(deserializer: D) -> Result<Vec<Pipeline>, D::Error>
where
    D: Deserializer<'de>
{
    struct PipelinesVisitor;

    impl<'de> Visitor<'de> for PipelinesVisitor {
        type Value = Vec<Pipeline>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an array of pipelines")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>
        {
            let mut pipelines: Vec<Pipeline> = Vec::new();

            while let Some(pipeline) = seq.next_element::<IntermediatePipeline>()? {
                match Pipeline::try_from(pipeline) {
                    Ok(pipeline) => pipelines.push(pipeline),
                    Err(error) => return Err(de::Error::invalid_value(Unexpected::Str(&error), &"a valid stage"))
                }
            }
            
            Ok(pipelines)
        }
    }

    deserializer.deserialize_seq(PipelinesVisitor)
}

impl Project {
    pub fn update_branches(&mut self, branches: Vec<Branch>) {
        self.branches = branches;
    }
}
