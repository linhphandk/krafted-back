use async_trait::async_trait;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use tracing::instrument;

use crate::shared::errors::{AppError, AppResult};

#[async_trait]
pub trait ImageStorage: Send + Sync {
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> AppResult<String>;
    async fn delete(&self, bucket: &str, key: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct S3ImageStorage {
    client: Client,
}

impl S3ImageStorage {
    pub async fn new(endpoint: Option<String>, region: Option<String>) -> Self {
        let loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
        let loader = match endpoint {
            Some(url) => loader.endpoint_url(url),
            None => loader,
        };
        let loader = match region {
            Some(r) => loader.region(Region::new(r)),
            None => loader,
        };
        let config = loader.load().await;
        let client = Client::new(&config);
        Self { client }
    }
}

#[async_trait]
impl ImageStorage for S3ImageStorage {
    #[instrument(skip(self, data), fields(bucket, key))]
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> AppResult<String> {
        let body = ByteStream::from(data);
        self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "S3 upload failed");
                AppError::Internal
            })?;

        let url = format!("{}/{}", bucket, key);
        Ok(url)
    }

    #[instrument(skip(self), fields(bucket, key))]
    async fn delete(&self, bucket: &str, key: &str) -> AppResult<()> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "S3 delete failed");
                AppError::Internal
            })?;
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::sync::Mutex;

    pub struct MockImageStorage {
        pub uploads: Mutex<Vec<(String, String, Vec<u8>, String)>>,
        pub deletes: Mutex<Vec<(String, String)>>,
    }

    impl MockImageStorage {
        pub fn new() -> Self {
            Self {
                uploads: Mutex::new(Vec::new()),
                deletes: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ImageStorage for MockImageStorage {
        async fn upload(
            &self,
            bucket: &str,
            key: &str,
            data: Vec<u8>,
            _content_type: &str,
        ) -> AppResult<String> {
            self.uploads.lock().unwrap().push((
                bucket.to_string(),
                key.to_string(),
                data,
                _content_type.to_string(),
            ));
            Ok(format!("http://mock/{}/{}", bucket, key))
        }

        async fn delete(&self, bucket: &str, key: &str) -> AppResult<()> {
            self.deletes
                .lock()
                .unwrap()
                .push((bucket.to_string(), key.to_string()));
            Ok(())
        }
    }
}
