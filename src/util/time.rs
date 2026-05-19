use std::time::{
   SystemTime,
   UNIX_EPOCH,
};

pub fn now_unix_secs() -> u64 {
   SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_or(0, |d| d.as_secs())
}
