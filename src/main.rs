use async_std::task;
use dotenv;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::prelude::ChannelId;
use serenity::prelude::*;
use std::time::{Duration, SystemTime};

mod ping;
use ping::ping_mumble;

const DISCORD_TOKEN_KEY: &str = "DISCORD_TOKEN";
const TARGET_CHANNEL_KEY: &str = "TARGET_CHANNEL_ID";
const LINK_KEY: &str = "CUSTOM_LINK";
const PING_INTERVAL: u64 = 5 * 60;

async fn update_channel_name(ctx: &Context, msg: &str, target_channel: Option<Box<ChannelId>>) {
    match target_channel {
        Some(target_channel) => {
            target_channel.edit(&ctx.http, |c| c.name(msg)).await.ok();
        }
        None => {}
    }
}

async fn update_mumble_user_count(ctx: &Context) {
    let mut online = 0;
    let mut last_update = SystemTime::now();
    loop {
        task::sleep(Duration::from_secs(1)).await;
        let res = ping_mumble().await.expect("failed to get");
        if online == 0 && res.users > 0
            || SystemTime::now()
                .duration_since(last_update)
                .expect("Unable to get current time")
                .as_secs()
                > PING_INTERVAL
        {
            online = res.users;
            last_update = SystemTime::now();
            update_channel_name(
                &ctx,
                format!("Online: {}", res.users).as_str(),
                Some(Box::new(ChannelId(
                    dotenv::var(TARGET_CHANNEL_KEY)
                        .expect("Please enter the ID for the target channel")
                        .parse::<u64>()
                        .unwrap(),
                ))),
            )
            .await;
        }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    // TODO: Enable users to add simple commands in .env
    async fn message(&self, ctx: Context, msg: Message) {
        match msg.content.as_str() {
            "!mumble" => {
                if let Err(why) = msg
                    .channel_id
                    .say(
                        &ctx.http,
                        dotenv::var(LINK_KEY).expect("Please enter a link to the server"),
                    )
                    .await
                {
                    eprintln!("Error sending message: {:?}", why);
                }
            }
            "!online" => {
                let res = ping_mumble().await.expect("failed to get");
                if let Err(why) = msg.channel_id.say(&ctx.http, res.users).await {
                    eprintln!("Error sending message: {:?}", why);
                }
            }
            &_ => {}
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        update_mumble_user_count(&ctx).await;
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = dotenv::var(DISCORD_TOKEN_KEY).expect("Expected token in .env");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}
