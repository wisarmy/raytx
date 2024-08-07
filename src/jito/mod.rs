use std::sync::LazyLock;

use crate::get_env_var;

pub mod api;
pub mod ws;

static BLOCK_ENGINE_URL: LazyLock<String> = LazyLock::new(|| get_env_var("JITO_BLOCK_ENGINE_URL"));
static TIP_STREAM_URL: LazyLock<String> = LazyLock::new(|| get_env_var("JITO_TIP_STREAM_URL"));
