use lazy_static::lazy_static;
use serenity::{
    prelude::*,
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
    },
    framework::standard::{
    CommandResult,
    StandardFramework,
        macros::{command, group},
    },
};
use regex::Regex;
use std::process::Stdio;
use tokio::io::{BufReader, AsyncBufReadExt};

struct SenderKey;
impl TypeMapKey for SenderKey {
    type Value = tokio::task::JoinHandle<()>;
}
struct FNameKey;
impl TypeMapKey for FNameKey {
    type Value = String;
}
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected", ready.user.name);
    }
}
#[group]
#[commands(start, stop)]
struct General;

lazy_static!{
    static ref RE: Regex = Regex::new(".*?: (<.*?> .*)$").unwrap();
}
#[command]
#[only_in(guilds)]
async fn start(ctx: &Context, msg: &Message) -> CommandResult {
    let fname = ctx.data.read().await.get::<FNameKey>().unwrap().clone();
    let cid=msg.channel_id;
    let http = ctx.http.clone();
    let mut log = tokio::process::Command::new("tail")
        .arg("-Fn0")
        .arg(&fname)
        .stdout(Stdio::piped())
        .spawn().unwrap();
    let stdout = log.stdout.take().unwrap();
    if let Some(old) = ctx.data.write().await.remove::<SenderKey>() {
        old.abort();
    }
    let handle  = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Some(line) = reader.next_line().await.unwrap() {
            if let Some(caps) = RE.captures(&line) {
                cid.say(&http, &caps[1]).await.unwrap();
            }
        }
    });
    ctx.data.write().await.insert::<SenderKey>(handle);
    msg.channel_id.say(&ctx, "start sending chat").await?;
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let x = ctx.data.write().await.remove::<SenderKey>();
    if let Some(handle) = x {
        handle.abort();
    }
    msg.channel_id.say(&ctx, "stop sending chat").await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let file_name = std::env::args().nth(1).unwrap();
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let framework = StandardFramework::new()
        .configure(|c|c
            .with_whitespace(true)
            .prefix("!mc ")
        )
        .group(&GENERAL_GROUP);
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<FNameKey>(file_name)
        .await
        .unwrap();
    client.start().await.unwrap();
}
