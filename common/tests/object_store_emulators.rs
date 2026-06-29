use std::env;
use std::error::Error;

use bytes::Bytes;
use common::storage::config::SlateDbStorageConfig;
use common::{
    AzureObjectStoreConfig, GcpObjectStoreConfig, ObjectStoreConfig, PutRecordOp, Record,
    StorageBuilder, StorageConfig, create_object_store,
};
use slatedb::object_store::{PutPayload, path::Path};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn Error + Send + Sync>>;

const GCP_ENDPOINT_ENV: &str = "OPENDATA_GCP_EMULATOR_ENDPOINT";
const GCP_BUCKET_ENV: &str = "OPENDATA_GCP_EMULATOR_BUCKET";
const GCS_BUCKET_ENV: &str = "OPENDATA_GCS_BUCKET";
const AZURE_ENDPOINT_ENV: &str = "OPENDATA_AZURE_EMULATOR_ENDPOINT";
const AZURE_CONTAINER_ENV: &str = "OPENDATA_AZURE_EMULATOR_CONTAINER";
const AZURE_ACCOUNT_ENV: &str = "OPENDATA_AZURE_EMULATOR_ACCOUNT";
const AZURE_ACCESS_KEY_ENV: &str = "OPENDATA_AZURE_EMULATOR_ACCESS_KEY";
const AZURITE_ACCESS_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";

#[tokio::test]
#[ignore = "requires a GCS emulator compatible with object_store's XML API"]
async fn should_put_get_and_delete_object_in_gcs_emulator() -> TestResult {
    // given
    let endpoint = required_env(GCP_ENDPOINT_ENV)?;
    let bucket = required_env(GCP_BUCKET_ENV)?;
    let config = ObjectStoreConfig::Gcp(GcpObjectStoreConfig {
        bucket,
        base_url: Some(endpoint),
        skip_signature: true,
    });
    let location = unique_location("gcs-emulator");
    let expected = Bytes::from_static(b"opendata gcp emulator e2e");

    // when
    let store = create_object_store(&config)?;
    store
        .put(&location, PutPayload::from_bytes(expected.clone()))
        .await?;
    let actual = store.get(&location).await?.bytes().await?;
    store.delete(&location).await?;

    // then
    assert_eq!(actual, expected);
    assert!(store.get(&location).await.is_err());

    Ok(())
}

#[tokio::test]
#[ignore = "requires OPENDATA_GCS_BUCKET and Google ADC credentials"]
async fn should_roundtrip_slatedb_storage_with_gcs() -> TestResult {
    // given
    let bucket = required_env(GCS_BUCKET_ENV)?;
    let config = slatedb_config(ObjectStoreConfig::Gcp(GcpObjectStoreConfig {
        bucket,
        base_url: None,
        skip_signature: false,
    }));
    let key = unique_key("gcs");
    let expected = Bytes::from_static(b"opendata gcs e2e");

    // when
    let storage = StorageBuilder::new(&config).await?.build().await?;
    storage
        .put(vec![PutRecordOp::new(Record::new(
            key.clone(),
            expected.clone(),
        ))])
        .await?;
    storage.flush().await?;
    let actual = storage.get(key.clone()).await?;
    storage.close().await?;

    // then
    assert_eq!(actual.map(|record| record.value), Some(expected));

    Ok(())
}

#[tokio::test]
#[ignore = "requires Azurite endpoint, container, account, and access key env vars"]
async fn should_roundtrip_slatedb_storage_with_azurite() -> TestResult {
    // given
    let endpoint = optional_env(
        AZURE_ENDPOINT_ENV,
        "http://127.0.0.1:10000/devstoreaccount1",
    );
    let container = optional_env(AZURE_CONTAINER_ENV, "opendata-e2e-azure");
    let account = optional_env(AZURE_ACCOUNT_ENV, "devstoreaccount1");
    let access_key = optional_env(AZURE_ACCESS_KEY_ENV, AZURITE_ACCESS_KEY);
    let config = slatedb_config(ObjectStoreConfig::Azure(AzureObjectStoreConfig {
        account: Some(account),
        container,
        endpoint: Some(endpoint),
        access_key: Some(access_key),
        allow_http: true,
        skip_signature: false,
    }));
    let key = unique_key("azurite");
    let expected = Bytes::from_static(b"opendata azure emulator e2e");

    // when
    let storage = StorageBuilder::new(&config).await?.build().await?;
    storage
        .put(vec![PutRecordOp::new(Record::new(
            key.clone(),
            expected.clone(),
        ))])
        .await?;
    storage.flush().await?;
    let actual = storage.get(key.clone()).await?;
    storage.close().await?;

    // then
    assert_eq!(actual.map(|record| record.value), Some(expected));

    Ok(())
}

fn required_env(name: &str) -> Result<String, env::VarError> {
    env::var(name)
}

fn optional_env(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn slatedb_config(object_store: ObjectStoreConfig) -> StorageConfig {
    StorageConfig::SlateDb(SlateDbStorageConfig {
        path: format!("opendata-e2e/{}", Uuid::new_v4()),
        object_store,
        settings_path: None,
        block_cache: None,
        meta_cache: None,
    })
}

fn unique_key(prefix: &str) -> Bytes {
    Bytes::from(format!("e2e/{}/{}", prefix, Uuid::new_v4()))
}

fn unique_location(prefix: &str) -> Path {
    Path::from(format!("e2e/{}/{}", prefix, Uuid::new_v4()))
}
