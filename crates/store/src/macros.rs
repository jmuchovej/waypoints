#[macro_export]
macro_rules! impl_timestamps {
    ($active_model:ty) => {
        impl $active_model {
            pub fn set_timestamps(&mut self, insert: bool) {
                use chrono::Local;
                use sea_orm::ActiveValue::{NotSet, Set};
                let now = Local::now().naive_utc();
                // On insert, only set created_at if not already explicitly set.
                // This allows callers to supply a frontend-sourced timestamp.
                if insert && matches!(self.created_at, NotSet) {
                    self.created_at = Set(now.clone());
                }
                self.updated_at = Set(now);
            }
        }
    };
}
