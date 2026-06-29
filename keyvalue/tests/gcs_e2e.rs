use bytes::Bytes;
use common::StorageConfig;
use common::storage::config::{GcpObjectStoreConfig, ObjectStoreConfig, SlateDbStorageConfig};
use keyvalue::{Config, KeyValueDb, KeyValueRead};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[tokio::test]
#[ignore = "requires OPENDATA_GCS_BUCKET and Google ADC credentials"]
async fn should_roundtrip_keyvalue_with_gcs() {
    let kv = KeyValueDb::open(Config {
        storage: gcs_storage_config("kv"),
    })
    .await
    .unwrap();
    let key = Bytes::from("smoke:key");
    let value = Bytes::from("smoke-value");

    kv.put(key.clone(), value.clone()).await.unwrap();
    kv.flush().await.unwrap();

    assert_eq!(kv.get(key).await.unwrap(), Some(value));
    let mut iter = kv
        .scan(Bytes::from("smoke:")..Bytes::from("smoke;"))
        .await
        .unwrap();
    assert!(iter.next().await.unwrap().is_some());
    kv.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires OPENDATA_GCS_BUCKET and Google ADC credentials"]
async fn should_write_100k_keyvalue_records_with_gcs() {
    let kv = KeyValueDb::open(Config {
        storage: gcs_storage_config("kv-100k"),
    })
    .await
    .unwrap();
    let started = Instant::now();

    for i in 0..100_000 {
        kv.put(
            Bytes::from(format!("load:{i:06}")),
            Bytes::from(format!("value-{i}")),
        )
        .await
        .unwrap();
    }
    kv.flush().await.unwrap();

    let mut count = 0;
    let mut iter = kv
        .scan(Bytes::from("load:")..Bytes::from("load;"))
        .await
        .unwrap();
    while iter.next().await.unwrap().is_some() {
        count += 1;
    }
    println!("keyvalue_100k_elapsed_ms={}", started.elapsed().as_millis());
    assert_eq!(count, 100_000);
    kv.close().await.unwrap();
}

fn gcs_storage_config(prefix: &str) -> StorageConfig {
    StorageConfig::SlateDb(SlateDbStorageConfig {
        path: format!("opendata-e2e/{prefix}/{}", unique_suffix()),
        object_store: ObjectStoreConfig::Gcp(GcpObjectStoreConfig {
            bucket: std::env::var("OPENDATA_GCS_BUCKET").expect("OPENDATA_GCS_BUCKET must be set"),
            base_url: None,
            skip_signature: false,
        }),
        settings_path: None,
        block_cache: None,
        meta_cache: None,
    })
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
