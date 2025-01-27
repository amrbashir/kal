use std::future::Future;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use wry::http::header::CONTENT_TYPE;
use wry::http::Request;
use wry::WebViewId;

use super::IconType;
use crate::utils::PathExt;
use crate::webview_window::ProtocolResult;

pub struct Service {
    pub icons_dir: PathBuf,
}

impl Service {
    pub const PROTOCOL_NAME: &str = "kalicon";

    pub fn new(kal_data_dir: &Path) -> Self {
        Self {
            icons_dir: kal_data_dir.join("icons"),
        }
    }

    /// `kalicon://` protocol
    pub fn protocol(
        self: Arc<Self>,
        webview_id: WebViewId<'_>,
        request: Request<Vec<u8>>,
    ) -> impl Future<Output = ProtocolResult> {
        let webview_id = webview_id.to_string();

        async move {
            let span = tracing::trace_span!("protocol::kalicon", ?webview_id, ?request);
            let _enter = span.enter();

            let path = &request.uri().path()[1..];
            let path_str = percent_encoding::percent_decode_str(path).decode_utf8()?;
            let path = dunce::canonicalize(PathBuf::from(&*path_str));

            let query = request.uri().query();
            let icon_type = query.map(IconType::from_str).transpose()?;

            let bytes = match icon_type {
                Some(IconType::Path) => smol::fs::read(path?).await?,

                Some(IconType::Overlay) => {
                    let (bottom, top) = path_str.split_once("<<>>").unwrap();

                    let bottom = dunce::canonicalize(bottom)?;
                    let top = dunce::canonicalize(top)?;

                    let hashed = format!("{}.{}", bottom.to_hash(), top.to_hash());
                    let out = self.icons_dir.join(hashed).with_extension("png");

                    super::extract_overlayed_cached(bottom, top, &out)?;

                    smol::fs::read(out).await?
                }

                _ => {
                    let path = path?;

                    let hashed = path.to_hash();
                    let out = self.icons_dir.join(hashed).with_extension("png");

                    super::extract_cached(&path, &out)?;

                    smol::fs::read(out).await?
                }
            };

            crate::ipc::response::base()
                .header(CONTENT_TYPE, "image/png")
                .body(bytes.into())
                .map_err(Into::into)
        }
    }
}
