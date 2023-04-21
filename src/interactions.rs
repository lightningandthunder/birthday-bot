pub mod interactions {
    use crate::constants::constants::{DATE_FORMAT, MSG_PREFIX};
    use crate::dates::dates;
    use crate::db::db::{self, create_pool};
    use chrono::{Duration, NaiveDate};
    use rand::distributions::{Distribution, Uniform};
    use regex::Regex;
    use serenity::model::prelude::Message;

    fn msg_contains<'a>(msg: &'a str, pattern: &'a str) -> Option<&'a str> {
        let re = Regex::new(pattern).unwrap();
        let _match = re.find(msg);
        match _match {
            Some(m) => Some(m.as_str()),
            None => None,
        }
    }

    pub async fn eval_message(msg: &Message) -> Result<Option<String>, String> {
        let pool = create_pool().await?;

        let msg_body = &msg.content;

        if !msg_body.contains(MSG_PREFIX) {
            return Ok(None);
        }

        let user_id = msg.author.id.to_string();

        // !bday set <date string>
        if let Some(date_part) = msg_contains(msg_body.as_str(), r"(?i)\sset\s[0-9]{1,2}/[0-9]{1,2}/[0-9]{4}")
        {
            let bday_date = dates::parse(date_part)?;
            return match db::save_bday(&pool, user_id, bday_date.to_string()).await {
                Ok(_) => Ok(Some(format!("Saved your bday: {}", bday_date.to_string()))),
                Err(err) => Err(err.to_string()),
            };
        }

        // !bday me
        if let Some(_) = msg_contains(msg_body.as_str(), r"(?i)\s+me") {
            let user = db::get_user_by_id(&pool, user_id.as_str()).await?;
            return match user {
                Some(u) => {
                    return Ok(Some(
                        format!("Look, it's u: {}\nand ur bday:{:?}\nand who ur subscribed to: {:?}\nand ur notification time: {:?}", 
                        u.id, 
                        u.birthdate, 
                        u.subscribed_to, 
                        u.subscribed_to_all,
                        )
                ));
                },
                None => Err(String::from("I'm sorry, you don't appear to exist! O_O;")),
            };
        }
        // !bday sub <user ID>
        // !bday sub all
        // !bday unsub <user ID>
        // !bday unsub all

        // !bday pog
        if let Some(_) = msg_contains(msg_body.as_str(), r"(?i)^\s+pog") {
            let pog = random_pog_count();
            let response = match pog {
                -5..=99 => format!("{} got {} poggies", msg.author.name, pog),
                _ => format!(
                    "wow!!! {} got {} poggies!!! bonky. i'm pogging out loud rn",
                    msg.author.name, pog
                ),
            };
            return Ok(Some(response));
        }

        Ok(None)
    }

    fn random_pog_count() -> i32 {
        let mut rng = rand::thread_rng();
        let d100 = Uniform::from(-5..101);
        let d25 = Uniform::from(1..26);
        let val = d100.sample(&mut rng);

        match val {
            -5..=95 => val,
            _ => val + d25.sample(&mut rng),
        }
    }

    pub async fn register_bday(current_user: String, bday: NaiveDate) -> Result<(), String> {
        let pool = create_pool().await?;
        let bday_string = bday.format(DATE_FORMAT).to_string();

        db::save_bday(&pool, current_user, bday_string).await
    }

    pub async fn subscribe_to_bday(
        current_user: String,
        target_user: String,
    ) -> Result<(), String> {
        let pool = create_pool().await?;

        db::add_bday_subscription(&pool, current_user, target_user).await
    }

    pub fn subscribe_to_all_bdays(
        current_user: String,
        notification_time: Duration,
    ) -> Result<Vec<String>, String> {
        // Probably want to return a list of who you subscribed to
        // also probably want to just have the db do this

        unimplemented!();
    }

    pub fn alert_bday(target_user: String, bday_user: String) -> Result<(), String> {
        unimplemented!();
    }
}
