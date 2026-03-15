/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Kafka `DataConnection` implementation.

use crate::continuous::datasource::{ConnectionDescriptor, DataConnection, DataConnectionError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::any::Any;

/// Kafka connection configuration returned by `connect()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConnectionConfig {
    pub brokers: Vec<String>,
    pub topic: String,
    pub partition: Option<i32>,
    pub consumer_group: Option<String>,
}

/// A Kafka data connection for continuous scheduling.
#[derive(Debug, Clone)]
pub struct KafkaConnection {
    pub brokers: Vec<String>,
    pub topic: String,
    pub partition: Option<i32>,
    pub consumer_group: Option<String>,
}

impl KafkaConnection {
    pub fn new(brokers: Vec<String>, topic: &str) -> Self {
        Self {
            brokers,
            topic: topic.to_string(),
            partition: None,
            consumer_group: None,
        }
    }

    pub fn with_partition(mut self, partition: i32) -> Self {
        self.partition = Some(partition);
        self
    }

    pub fn with_consumer_group(mut self, group: &str) -> Self {
        self.consumer_group = Some(group.to_string());
        self
    }
}

impl DataConnection for KafkaConnection {
    fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
        Ok(Box::new(KafkaConnectionConfig {
            brokers: self.brokers.clone(),
            topic: self.topic.clone(),
            partition: self.partition,
            consumer_group: self.consumer_group.clone(),
        }))
    }

    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "kafka".to_string(),
            location: format!("{}/{}", self.brokers.join(","), self.topic),
        }
    }

    fn system_metadata(&self) -> serde_json::Value {
        json!({
            "brokers": self.brokers,
            "topic": self.topic,
            "partition": self.partition,
            "consumer_group": self.consumer_group,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kafka_descriptor() {
        let conn =
            KafkaConnection::new(vec!["broker1:9092".into(), "broker2:9092".into()], "events");
        let desc = conn.descriptor();
        assert_eq!(desc.system_type, "kafka");
        assert_eq!(desc.location, "broker1:9092,broker2:9092/events");
    }

    #[test]
    fn test_kafka_metadata() {
        let conn = KafkaConnection::new(vec!["broker:9092".into()], "clicks")
            .with_partition(3)
            .with_consumer_group("my_group");
        let meta = conn.system_metadata();
        assert_eq!(meta["topic"], "clicks");
        assert_eq!(meta["partition"], 3);
        assert_eq!(meta["consumer_group"], "my_group");
    }

    #[test]
    fn test_kafka_connect_returns_config() {
        let conn = KafkaConnection::new(vec!["broker:9092".into()], "events");
        let handle = conn.connect().unwrap();
        let config = handle.downcast::<KafkaConnectionConfig>().unwrap();
        assert_eq!(config.topic, "events");
        assert_eq!(config.brokers, vec!["broker:9092"]);
    }
}
