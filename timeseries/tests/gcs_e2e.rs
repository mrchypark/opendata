use common::StorageConfig;
use common::storage::config::{GcpObjectStoreConfig, ObjectStoreConfig, SlateDbStorageConfig};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use timeseries::{Config, QueryValue, Series, TimeSeriesDb};

#[tokio::test]
#[ignore = "requires OPENDATA_GCS_BUCKET and Google ADC credentials"]
async fn should_roundtrip_timeseries_with_gcs() {
    let ts = TimeSeriesDb::open(Config {
        storage: gcs_storage_config("ts"),
        ..Default::default()
    })
    .await
    .unwrap();
    let t = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);

    ts.write(vec![
        Series::builder("opendata_gcs_smoke")
            .label("db", "timeseries")
            .sample(1_700_000_000_000, 42.0)
            .build(),
    ])
    .await
    .unwrap();
    ts.flush().await.unwrap();

    match ts.query("opendata_gcs_smoke", Some(t)).await.unwrap() {
        QueryValue::Vector(samples) => {
            assert_eq!(samples.len(), 1);
            assert_eq!(samples[0].value, 42.0);
        }
        other => panic!("expected vector, got {other:?}"),
    }
    ts.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires OPENDATA_GCS_BUCKET and Google ADC credentials"]
async fn should_write_100k_timeseries_samples_with_gcs() {
    let ts = TimeSeriesDb::open(Config {
        storage: gcs_storage_config("ts-100k"),
        ..Default::default()
    })
    .await
    .unwrap();
    let base_ms = 1_700_000_000_000;
    let started = Instant::now();

    let series = (0..100)
        .map(|host| {
            let mut builder = Series::builder("opendata_gcs_load")
                .label("db", "timeseries")
                .label("host", format!("host-{host}"));
            for sample in 0..1000 {
                builder = builder.sample(base_ms + sample, sample as f64);
            }
            builder.build()
        })
        .collect();
    ts.write(series).await.unwrap();
    ts.flush().await.unwrap();

    let t = UNIX_EPOCH + Duration::from_millis((base_ms + 999) as u64);
    match ts.query("opendata_gcs_load", Some(t)).await.unwrap() {
        QueryValue::Vector(samples) => assert_eq!(samples.len(), 100),
        other => panic!("expected vector, got {other:?}"),
    }
    println!(
        "timeseries_100k_elapsed_ms={}",
        started.elapsed().as_millis()
    );
    ts.close().await.unwrap();
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
