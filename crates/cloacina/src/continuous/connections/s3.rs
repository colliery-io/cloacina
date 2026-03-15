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

//! S3 `DataConnection` implementation.

use crate::continuous::datasource::{ConnectionDescriptor, DataConnection, DataConnectionError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::any::Any;

/// S3 connection configuration returned by `connect()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ConnectionConfig {
    pub bucket: String,
    pub prefix: String,
    pub region: String,
}

/// An S3 data connection for continuous scheduling.
#[derive(Debug, Clone)]
pub struct S3Connection {
    pub bucket: String,
    pub prefix: String,
    pub region: String,
}

impl S3Connection {
    pub fn new(bucket: &str, prefix: &str, region: &str) -> Self {
        Self {
            bucket: bucket.to_string(),
            prefix: prefix.to_string(),
            region: region.to_string(),
        }
    }
}

impl DataConnection for S3Connection {
    fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
        Ok(Box::new(S3ConnectionConfig {
            bucket: self.bucket.clone(),
            prefix: self.prefix.clone(),
            region: self.region.clone(),
        }))
    }

    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "s3".to_string(),
            location: format!("s3://{}/{}", self.bucket, self.prefix),
        }
    }

    fn system_metadata(&self) -> serde_json::Value {
        json!({
            "bucket": self.bucket,
            "prefix": self.prefix,
            "region": self.region,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_descriptor() {
        let conn = S3Connection::new("my-bucket", "data/raw/", "us-east-1");
        let desc = conn.descriptor();
        assert_eq!(desc.system_type, "s3");
        assert_eq!(desc.location, "s3://my-bucket/data/raw/");
    }

    #[test]
    fn test_s3_metadata() {
        let conn = S3Connection::new("analytics", "events/2024/", "eu-west-1");
        let meta = conn.system_metadata();
        assert_eq!(meta["bucket"], "analytics");
        assert_eq!(meta["prefix"], "events/2024/");
        assert_eq!(meta["region"], "eu-west-1");
    }

    #[test]
    fn test_s3_connect_returns_config() {
        let conn = S3Connection::new("bucket", "prefix/", "us-west-2");
        let handle = conn.connect().unwrap();
        let config = handle.downcast::<S3ConnectionConfig>().unwrap();
        assert_eq!(config.bucket, "bucket");
        assert_eq!(config.region, "us-west-2");
    }
}
