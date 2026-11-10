//! TaskState Custom Serialization and Deserialization (§ Phase 3.2 DDD Foundation)

use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use super::state::TaskState;

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum TaggedSerialize<'a> {
    Idle,
    Running {
        model: &'a str,
    },
    WaitingSubtask {
        subtask_id: &'a str,
    },
    WaitingSubagent,
    Done {
        #[serde(skip_serializing_if = "Option::is_none")]
        verdict: Option<&'a str>,
    },
}

impl Serialize for TaskState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TaskState::Idle => TaggedSerialize::Idle.serialize(serializer),
            TaskState::Running { model } => TaggedSerialize::Running {
                model: model.as_str(),
            }
            .serialize(serializer),
            TaskState::WaitingSubtask { subtask_id } => TaggedSerialize::WaitingSubtask {
                subtask_id: subtask_id.as_str(),
            }
            .serialize(serializer),
            TaskState::WaitingSubagent => TaggedSerialize::WaitingSubagent.serialize(serializer),
            TaskState::Done { verdict } => TaggedSerialize::Done {
                verdict: verdict.as_deref(),
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for TaskState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "status", rename_all = "snake_case")]
        enum Tagged {
            Idle,
            Running {
                #[serde(default = "default_model")]
                model: String,
            },
            WaitingSubtask {
                #[serde(alias = "subtaskId", default)]
                subtask_id: String,
            },
            WaitingSubagent,
            Done {
                #[serde(default)]
                verdict: Option<String>,
            },
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Tagged(Tagged),
            Str(String),
        }

        fn default_model() -> String {
            "default".to_string()
        }

        match Helper::deserialize(deserializer)? {
            Helper::Tagged(tagged) => match tagged {
                Tagged::Idle => Ok(TaskState::Idle),
                Tagged::Running { model } => Ok(TaskState::Running { model }),
                Tagged::WaitingSubtask { subtask_id } => Ok(TaskState::WaitingSubtask { subtask_id }),
                Tagged::WaitingSubagent => Ok(TaskState::WaitingSubagent),
                Tagged::Done { verdict } => Ok(TaskState::Done { verdict }),
            },
            Helper::Str(s) => TaskState::from_str(&s).map_err(serde::de::Error::custom),
        }
    }
}
