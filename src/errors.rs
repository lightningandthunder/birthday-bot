pub mod errors {
    use quick_error::quick_error;
    use sqlx;

    #[derive(Debug)]
    pub struct DataError;

    quick_error! {
        #[derive(Debug)]
        pub enum SqlError {
            DbError (err: sqlx::Error) {
                from()
            }
            UserNotFoundError {
                from()
            }
            FieldWasEmptyError {
                from()
            }
        }
    }
}