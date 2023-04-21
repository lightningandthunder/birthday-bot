pub mod constants;
pub mod dates;
pub mod db;
pub mod errors;
pub mod interactions;

use std::env;

use serenity::async_trait;
use serenity::framework::standard::StandardFramework;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use dotenv::dotenv;

use crate::db::db::init;
use crate::interactions::interactions::eval_message;

struct Handler;

async fn send(msg: Message, ctx: &Context, res: String) {
    let message = msg
        .channel_id
        .send_message(&ctx.http, |m| m.content(res))
        .await;

    if let Err(why) = message {
        println!("Error sending message: {:?}", why);
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.is_own(&ctx.cache) {
            return;
        }

        match eval_message(&msg).await {
            Ok(opt) => match opt {
                Some(res) => send(msg, &ctx, res).await,
                None => (),
            },
            Err(err) => send(msg, &ctx, err).await,
        };
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("No token found");

    init().await.unwrap();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .configure(|c| c.with_whitespace(false));

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}