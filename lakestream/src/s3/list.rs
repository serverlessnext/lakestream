use std::collections::HashMap;

use serde::Deserialize;

use super::bucket::{S3Bucket, S3Credentials};
use super::client::S3Client;
use super::config::update_config;
use crate::base::interfaces::ObjectStoreTrait;
use crate::http::requests::{http_get_request, http_get_request_with_headers};
use crate::utils::time::rfc3339_to_epoch;
use crate::{FileObject, ObjectStore, DEFAULT_AWS_REGION};

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ListAllMyBucketsResult {
    Buckets: Buckets,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Buckets {
    Bucket: Vec<Bucket>,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Bucket {
    Name: String,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ListBucketResult {
    Contents: Option<Vec<Content>>,
    CommonPrefixes: Option<Vec<CommonPrefix>>,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct Content {
    Key: String,
    LastModified: String,
    Size: u64,
    ETag: String,
}

// allow non snake case for the XML response
#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct CommonPrefix {
    Prefix: String,
}

pub fn list_files(
    s3_bucket: &S3Bucket,
    prefix: Option<&str>,
    recursive: bool,
    max_keys: Option<u32>,
) -> Vec<FileObject> {
    // The code from list_files() in s3/bucket.rs goes here
    let access_key = s3_bucket
        .config()
        .get("access_key")
        .expect("Missing access_key in the configuration");
    let secret_key = s3_bucket
        .config()
        .get("secret_key")
        .expect("Missing secret_key in the configuration");
    let mut region = s3_bucket
        .config()
        .get("region")
        .expect("Missing region in the configuration")
        .to_owned();

    let credentials =
        S3Credentials::new(String::from(access_key), String::from(secret_key));

    let mut attempt = 0;
    loop {
        let endpoint_url = format!(
            "https://{}.s3.{}.amazonaws.com:443",
            s3_bucket.name(),
            region
        );
        let mut s3_client =
            S3Client::new(endpoint_url, region.clone(), credentials.clone());
        let headers = s3_client
            .generate_list_objects_headers(prefix, max_keys)
            .unwrap();

        let result = http_get_request_with_headers(&s3_client.url(), &headers);

        match result {
            Ok((body, status, response_headers)) => {
                if status == 301 {
                    if let Some(new_region) =
                        response_headers.get("x-amz-bucket-region")
                    {
                        region = new_region.to_owned();
                        attempt += 1;
                        if attempt > 1 {
                            eprintln!("Error: Multiple redirect attempts");
                            return Vec::new();
                        }
                        continue;
                    } else {
                        eprintln!(
                            "Error: Redirect without x-amz-bucket-region \
                             header"
                        );
                        return Vec::new();
                    }
                }
                return match parse_file_objects(&body) {
                    Ok(file_objects) => file_objects,
                    Err(e) => {
                        eprintln!("Error listing objects: {}", e);
                        Vec::new()
                    }
                };
            }
            Err(e) => {
                eprintln!("Error in http_get_request: {}", e);
                return Vec::new();
            }
        }
    }
}

pub fn list_buckets(
    config: &HashMap<String, String>,
) -> Result<Vec<ObjectStore>, &'static str> {
    let updated_config = update_config(config)?;

    let region = updated_config
        .get("region")
        .cloned()
        .unwrap_or_else(|| DEFAULT_AWS_REGION.to_string());
    let access_key = updated_config
        .get("access_key")
        .expect("Missing access_key in the configuration");
    let secret_key = updated_config
        .get("secret_key")
        .expect("Missing secret_key in the configuration");

    let credentials =
        S3Credentials::new(String::from(access_key), String::from(secret_key));
    let endpoint_url = format!("https://s3.{}.amazonaws.com", region);
    let mut s3_client =
        S3Client::new(endpoint_url, String::from(region), credentials);
    let headers = s3_client.generate_list_buckets_headers().unwrap();

    let result = http_get_request(&s3_client.url(), &headers);

    let bucket_objects = match result {
        Ok((body, _)) => {
            match parse_bucket_objects(&body, Some(config.clone())) {
                Ok(bucket_objects) => bucket_objects,
                Err(e) => {
                    eprintln!("Error listing buckets: {}", e);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            eprintln!("Error in http_get_request: {}", e);
            Vec::new()
        }
    };
    Ok(bucket_objects)
}

fn parse_bucket_objects(
    body: &str,
    config: Option<HashMap<String, String>>,
) -> Result<Vec<ObjectStore>, Box<dyn std::error::Error>> {
    let list_all_my_buckets_result: ListAllMyBucketsResult =
        serde_xml_rs::from_str(body)?;
    let object_stores: Vec<ObjectStore> = list_all_my_buckets_result
        .Buckets
        .Bucket
        .iter()
        .map(|bucket| {
            let name = bucket.Name.clone();
            let config = config.clone().unwrap_or_else(HashMap::new);
            ObjectStore::new(&format!("s3://{}", name), config).unwrap()
        })
        .collect();
    Ok(object_stores)
}

fn parse_file_objects(
    body: &str,
) -> Result<Vec<FileObject>, Box<dyn std::error::Error>> {
    let list_bucket_result: ListBucketResult = serde_xml_rs::from_str(body)?;
    let file_objects: Vec<FileObject> = list_bucket_result
        .Contents
        .unwrap_or_default()
        .iter()
        .map(|content| {
            FileObject::new(
                content.Key.clone(),
                content.Size,
                Some(rfc3339_to_epoch(content.LastModified.as_str()).unwrap()),
                Some(
                    [(
                        "ETag".to_string(),
                        content.ETag.trim_matches('"').to_string(),
                    )]
                    .iter()
                    .cloned()
                    .collect::<HashMap<String, String>>(),
                ),
            )
        })
        .collect();
    let common_prefixes: Vec<String> = list_bucket_result
        .CommonPrefixes
        .unwrap_or_default()
        .iter()
        .map(|common_prefix| common_prefix.Prefix.clone())
        .collect();
    let common_prefix_file_objects: Vec<FileObject> = common_prefixes
        .iter()
        .map(|prefix| {
            FileObject::new(
                prefix.clone(), // Set the key to the prefix
                0,              // Set the size to 0
                None,           // Set the modified timestamp to None
                None,           // Set the tags to None
            )
        })
        .collect();
    let all_file_objects: Vec<FileObject> =
        [&file_objects[..], &common_prefix_file_objects[..]].concat();
    Ok(all_file_objects)
}
