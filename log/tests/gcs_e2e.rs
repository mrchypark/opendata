use bytes::Bytes;
use common::StorageConfig;
use common::storage::config::{GcpObjectStoreConfig, ObjectStoreConfig, SlateDbStorageConfig};
use log::{Config, LogDb, LogRead, Record};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[tokio::test]
#[ignore = "requires OPENDATA_GCS_BUCKET and Google ADC credentials"]
async fn should_roundtrip_log_with_gcs() {
    let log = LogDb::open(Config {
        storage: gcs_storage_config("log"),
        ..Default::default()
    })
    .await
    .unwrap();
    let key = Bytes::from("smoke-log");

    log.try_append(vec![
        Record {
            key: key.clone(),
            value: Bytes::from("one"),
        },
        Record {
            key: key.clone(),
            value: Bytes::from("two"),
        },
    ])
    .await
    .unwrap();
    log.flush().await.unwrap();

    let mut iter = log.scan(key, ..).await.unwrap();
    assert_eq!(
        iter.next().await.unwrap().unwrap().value,
        Bytes::from("one")
    );
    assert_eq!(
        iter.next().await.unwrap().unwrap().value,
        Bytes::from("two")
    );
    assert!(iter.next().await.unwrap().is_none());
    log.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires OPENDATA_GCS_BUCKET and Google ADC credentials"]
async fn should_append_100k_log_records_with_gcs() {
    let log = LogDb::open(Config {
        storage: gcs_storage_config("log-100k"),
        ..Default::default()
    })
    .await
    .unwrap();
    let key = Bytes::from("load-log");
    let started = Instant::now();

    for batch in 0..100 {
        let records = (0..1000)
            .map(|i| Record {
                key: key.clone(),
                value: Bytes::from(format!("value-{}", batch * 1000 + i)),
            })
            .collect();
        log.try_append(records).await.unwrap();
    }
    log.flush().await.unwrap();

    let mut count = 0;
    let mut iter = log.scan(key, ..).await.unwrap();
    while iter.next().await.unwrap().is_some() {
        count += 1;
    }
    println!("log_100k_elapsed_ms={}", started.elapsed().as_millis());
    assert_eq!(count, 100_000);
    log.close().await.unwrap();
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
