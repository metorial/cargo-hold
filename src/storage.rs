use bytes::Bytes;
use reqwest::Client;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Storage operation failed: {0}")]
    OperationFailed(String),
}

#[derive(Clone)]
pub struct ObjectStorageClient {
    client: Client,
    base_url: String,
    bucket: String,
}

impl ObjectStorageClient {
    pub fn new(base_url: String, bucket: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            bucket,
        }
    }

    pub async fn upload(
        &self,
        key: &str,
        data: Bytes,
        content_type: Option<&str>,
    ) -> Result<(), StorageError> {
        let url = format!(
            "{}/buckets/{}/objects/{}",
            self.base_url, self.bucket, key
        );

        let mut req = self.client.put(&url).body(data);

        if let Some(ct) = content_type {
            req = req.header("Content-Type", ct);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(StorageError::OperationFailed(format!(
                "Upload failed with status: {}",
                response.status()
            )));
        }

        Ok(())
    }

    pub async fn download(&self, key: &str) -> Result<Bytes, StorageError> {
        let url = format!(
            "{}/buckets/{}/objects/{}",
            self.base_url, self.bucket, key
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(StorageError::OperationFailed(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        Ok(response.bytes().await?)
    }

    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let url = format!(
            "{}/buckets/{}/objects/{}",
            self.base_url, self.bucket, key
        );

        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() && response.status().as_u16() != 404 {
            return Err(StorageError::OperationFailed(format!(
                "Delete failed with status: {}",
                response.status()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upload_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("PUT", "/buckets/test-bucket/objects/test-key")
            .with_status(200)
            .create();

        let client = ObjectStorageClient::new(server.url(), "test-bucket".to_string());
        let result = client
            .upload("test-key", Bytes::from("test data"), None)
            .await;

        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_upload_with_content_type() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("PUT", "/buckets/test-bucket/objects/test-key")
            .match_header("Content-Type", "image/png")
            .with_status(200)
            .create();

        let client = ObjectStorageClient::new(server.url(), "test-bucket".to_string());
        let result = client
            .upload("test-key", Bytes::from("test data"), Some("image/png"))
            .await;

        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_upload_failure() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("PUT", "/buckets/test-bucket/objects/test-key")
            .with_status(500)
            .create();

        let client = ObjectStorageClient::new(server.url(), "test-bucket".to_string());
        let result = client
            .upload("test-key", Bytes::from("test data"), None)
            .await;

        mock.assert();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_download_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/buckets/test-bucket/objects/test-key")
            .with_status(200)
            .with_body("test data")
            .create();

        let client = ObjectStorageClient::new(server.url(), "test-bucket".to_string());
        let result = client.download("test-key").await;

        mock.assert();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Bytes::from("test data"));
    }

    #[tokio::test]
    async fn test_download_not_found() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/buckets/test-bucket/objects/test-key")
            .with_status(404)
            .create();

        let client = ObjectStorageClient::new(server.url(), "test-bucket".to_string());
        let result = client.download("test-key").await;

        mock.assert();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("DELETE", "/buckets/test-bucket/objects/test-key")
            .with_status(204)
            .create();

        let client = ObjectStorageClient::new(server.url(), "test-bucket".to_string());
        let result = client.delete("test-key").await;

        mock.assert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_not_found_ok() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("DELETE", "/buckets/test-bucket/objects/test-key")
            .with_status(404)
            .create();

        let client = ObjectStorageClient::new(server.url(), "test-bucket".to_string());
        let result = client.delete("test-key").await;

        mock.assert();
        assert!(result.is_ok());
    }
}
