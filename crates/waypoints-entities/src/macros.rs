#[macro_export]
macro_rules! impl_timestamps {
    ($active_model:ty) => {
        impl $active_model {
            pub fn set_timestamps(&mut self, insert: bool) {
                use chrono::Local;
                use sea_orm::ActiveValue::{NotSet, Set};
                let now = Local::now().naive_utc();
                self.created_at = match insert {
                    true => Set(now.clone()),
                    false => NotSet,
                };
                self.updated_at = Set(now);
            }
        }
    };
}
