use crate::response::send_response_file;
use crate::CBLTError;
use http::{Request, Response, StatusCode};
use std::path::{Component, Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::instrument;

#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
pub async fn file_directive<S>(
    root_path: Option<&str>,
    request: &Request<Vec<u8>>,
    socket: &mut S,
    req_opt: &Request<Vec<u8>>,
) -> Result<StatusCode, CBLTError>
where
    S: AsyncWriteExt + Unpin,
{
    match root_path {
        None => {
            return Err(CBLTError::ResponseError {
                details: "".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        Some(root) => {
            if let Some(mut file_path) = sanitize_path(
                Path::new(root),
                request.uri().path().trim_start_matches('/'),
            ) {
                if file_path.is_dir() {
                    file_path.push("index.html");
                }

                match File::open(&file_path).await {
                    Ok(file) => {
                        let content_length = file_size(&file).await;
                        let response = file_response(file, content_length);
                        send_response_file(socket, response, req_opt).await?;
                        return Ok(StatusCode::OK);
                    }
                    Err(_) => {
                        return Err(CBLTError::ResponseError {
                            details: "Not found".to_string(),
                            status_code: StatusCode::NOT_FOUND,
                        });
                    }
                }
            } else {
                return Err(CBLTError::DirectiveNotMatched);
            }
        }
    }
}

#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
async fn file_size(file: &File) -> u64 {
    let metadata = file.metadata().await.unwrap();
    metadata.len()
}

#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
fn file_response(file: File, content_length: u64) -> Response<File> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Length", content_length)
        .body(file)
        .unwrap()
}

#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all))]
fn sanitize_path(base_path: &Path, requested_path: &str) -> Option<PathBuf> {
    let mut full_path = base_path.to_path_buf();
    let requested_path = Path::new(requested_path);

    for component in requested_path.components() {
        match component {
            Component::Normal(segment) => full_path.push(segment),
            Component::RootDir | Component::Prefix(_) => return None,
            Component::ParentDir => {
                if !full_path.pop() {
                    return None;
                }
            }
            Component::CurDir => {}
        }
    }

    if full_path.starts_with(base_path) {
        Some(full_path)
    } else {
        None
    }
}
