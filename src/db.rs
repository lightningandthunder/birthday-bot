pub mod db {
    use crate::constants::constants::{DB_NAME, TABLE_NAME};
    use serde::{Deserialize, Serialize};
    use sqlx::migrate::MigrateDatabase;
    use sqlx::sqlite::{SqlitePoolOptions, SqliteRow, Sqlite};
    use sqlx::{Pool, Row};

    pub type UserArray = Vec<String>;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Clone)]
    pub struct DbUser {
        pub id: String,
        pub birthdate: Option<String>,
        pub subscribed_to: Option<UserArray>,
        pub subscribed_to_all: bool,
    }

    impl DbUser {
        pub fn birthdate_str(&self) -> Option<&str> {
            self.birthdate
                .as_ref()
                .and(Some(self.birthdate.as_ref().unwrap().as_str()))
        }

        pub fn subscription_string(&self) -> Option<String> {
            match self.subscribed_to {
                Some(ref subs) => Some(user_array_to_string(subs)),
                None => None,
            }
        }
    }

    fn map_sqlx_err(err: sqlx::Error) -> String {
        err.to_string()
    }

    fn string_to_user_array(s: &str) -> UserArray {
        serde_json::from_str(s).unwrap()
    }

    fn user_array_to_string(s: &UserArray) -> String {
        serde_json::to_string(s).unwrap()
    }

    pub async fn create_pool() -> Result<Pool<Sqlite>, String> {
        SqlitePoolOptions::new()
            .max_connections(4)
            .connect(DB_NAME)
            .await
            .map_err(map_sqlx_err)
    }

    pub async fn init() -> Result<(), String> {
        Sqlite::drop_database(DB_NAME).await.unwrap();
        if !Sqlite::database_exists(DB_NAME).await.unwrap_or(false) {
            Sqlite::create_database(DB_NAME).await.unwrap();
        }
        let pool = create_pool().await?;
        sqlx::query(
            "
            DROP TABLE IF EXISTS $1;
            CREATE TABLE IF NOT EXISTS $2 (
                user_id TEXT NOT NULL PRIMARY KEY,
                birthdate TEXT,
                subscribed_to JSON,
                subscribed_to_all INTEGER
            );",
        )
        .bind(TABLE_NAME)
        .bind(TABLE_NAME)
        .fetch_optional(&pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn user_exists(pool: &Pool<Sqlite>, user_id: &str) -> Result<bool, String> {
        let user_opt = get_user_by_id(pool, &user_id).await?;
        Ok(user_opt.is_some())
    }

    async fn ensure_user_exists(pool: &Pool<Sqlite>, user_id: &str) -> Result<(), String> {
        Ok(match user_exists(pool, &user_id).await? {
            true => Ok(()),
            false => Err(String::from("User not found")),
        }?)
    }

    async fn insert_user(pool: &Pool<Sqlite>, user: DbUser) -> Result<(), String> {
        let subs = user.subscription_string().as_deref();
        let result = sqlx::query(
            "
        INSERT INTO $1 (user_id, birthdate, subscribed_to, subscribed_to_all)
        VALUES($2, $3, $4, $5)",
        )
        .bind(TABLE_NAME)
        .bind(user.id.as_str())
        .bind(user.birthdate_str())
        .bind(user.subscription_string().as_deref().unwrap())
        .bind(user.subscribed_to_all)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    async fn update_user(pool: &Pool<Sqlite>, user: DbUser) -> Result<(), String> {
        let subs = user.subscription_string();
        let result = sqlx::query(
            "
        UPDATE $1
        SET birthdate=COALESCE($2, birthdate),
            subscribed_to=COALESCE($3, subscribed_to),
            subscribed_to_all=COALESCE($4, subscribed_to_all)
        WHERE user_id=$5
    ",
        )
        .bind(TABLE_NAME)
        .bind(&user.birthdate_str())
        .bind(subs)
        .bind(user.subscribed_to_all)
        .bind(&user.id)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    pub async fn save_bday(pool: &Pool<Sqlite>, current_user_id: String, bday: String) -> Result<(), String> {
        let user_option = get_user_by_id(pool, current_user_id.as_str()).await;

        match user_option {
            Ok(opt) => match opt {
                Some(mut u) => {
                    u.birthdate = Some(bday);
                    return update_user(pool, u).await;
                }
                None => {
                    let new_user = DbUser {
                        id: current_user_id,
                        birthdate: Some(bday),
                        subscribed_to: None,
                        subscribed_to_all: false,
                    };
                    return Ok(insert_user(pool, new_user).await?);
                }
            },
            Err(err) => return Err(err.to_string()),
        }
    }

    pub async fn get_all_user_ids() -> Result<Vec<String>, String> {
        let pool = create_pool().await?;

        let mut user_ids = vec![];
        sqlx::query(
            "
            SELECT user_id
            FROM $1
    ",
        )
        .bind(TABLE_NAME)
        .fetch_all(&pool)
        .await
        .and_then(|vec| {
            Ok({
                let x = vec.iter().try_for_each(|row| {
                    match row.try_get::<String, _>(0) {
                        Ok(s) => user_ids.push(s.to_owned()),
                        Err(err) => return Err(err.to_string()),
                    };
                    Ok(())
                });
            })
        })
        .map_err(map_sqlx_err)?;

        Ok(user_ids)
    }

    pub async fn get_user_by_id(pool: &Pool<Sqlite>, user_id: &str) -> Result<Option<DbUser>, String> {
        let result = sqlx::query(
            "
        SELECT user_id, birthdate, subscribed_to
        FROM $1
        WHERE user_id=$2
    ",
        )
        .bind(TABLE_NAME)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?
        .map(|row: SqliteRow| {
            // I'm sure there's a way to do this without all the map errors, but I don't know what it is!
            return Ok(DbUser {
                id: user_id.to_string(),
                birthdate: row.try_get(1).map_err(map_sqlx_err)?,
                subscribed_to: row
                    .try_get(2)
                    .and_then(|val| Ok(Some(string_to_user_array(val))))
                    .map_err(map_sqlx_err)?,
                subscribed_to_all: row
                    .try_get(3)
                    .and_then(|val: i32| match val {
                        0 => Ok(false),
                        _ => Ok(true),
                    })
                    .map_err(map_sqlx_err)?,
            });
        });

        match result {
            Some(row) => match row {
                Ok(db_user) => Ok(Some(db_user)),
                Err(err) => Err(err),
            },
            None => Ok(None),
        }
    }

    pub async fn add_bday_subscription(
        pool: &Pool<Sqlite>,
        current_user_id: String,
        bday_user_id: String,
    ) -> Result<(), String> {
        let mut current_user = match get_user_by_id(pool, &current_user_id.to_string()).await {
            Ok(opt) => match opt {
                Some(user) => user,
                None => return Err(String::from("No match found for current user")),
            },
            Err(err) => return Err(err.to_string()),
        };

        ensure_user_exists(pool, &bday_user_id).await?;

        if current_user.subscribed_to.as_ref().is_some() {
            current_user
                .subscribed_to
                .as_mut()
                .unwrap()
                .push(bday_user_id.clone());
        } else {
            current_user.subscribed_to = Some(vec![bday_user_id.clone()]);
        }

        let result = sqlx::query(
            "
            UPDATE $1
            SET subscribed_to=$2
            WHERE user_id=$3
        ",
        )
        .bind(TABLE_NAME)
        .bind(current_user.subscription_string().unwrap())
        .bind(current_user.id.as_str())
        .execute(pool)
        .await
        .map_err(|err| err.to_string())?;

        Ok(())
    }

    pub async fn set_subscribe_to_all(
        pool: &Pool<Sqlite>,
        current_user_id: String,
        subscribe: bool,
    ) -> Result<(), String> {
        ensure_user_exists(pool, &current_user_id).await?;

        let result = sqlx::query(
            "
            UPDATE $1
            SET subscribed_to_all = $2
            WHERE user_id=$3
        ",
        )
        .bind(TABLE_NAME)
        .bind(current_user_id)
        .bind(match subscribe {
            true => 1,
            false => 0,
        })
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }

    pub async fn remove_bday_subscription(
        pool: &Pool<Sqlite>,
        current_user_id: String,
        bday_user_id: String,
    ) -> Result<(), String> {
        ensure_user_exists(pool, &bday_user_id).await?;

        let mut current_user = get_user_by_id(pool, &current_user_id)
            .await?
            .ok_or(String::from("Current user ID doesn't exist"))?;

        if current_user.subscribed_to.is_none() {
            return Err(String::from("User is not subscribed to any birthdays."));
        }

        if let Ok(index) = current_user
            .subscribed_to
            .as_mut()
            .unwrap()
            .binary_search(&bday_user_id)
        {
            current_user.subscribed_to.as_mut().unwrap().remove(index);
        }

        sqlx::query(
            "
            UPDATE $1
            SET subscribed_to=$2
            WHERE user_id=$3
        ",
        )
        .bind(TABLE_NAME)
        .bind(current_user.subscription_string())
        .bind(current_user_id)
        .execute(pool)
        .await
        .map_err(map_sqlx_err)?;

        Ok(())
    }
}
