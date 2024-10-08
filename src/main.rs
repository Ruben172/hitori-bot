#![warn(clippy::pedantic)]
#![allow(
    clippy::similar_names,
    clippy::unreadable_literal,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::single_match_else,
    clippy::if_not_else,
    clippy::cast_lossless,
    clippy::module_name_repetitions
)]

mod commands;
mod tasks;
mod util;

use crate::tasks::task_handler;
use dotenvy::dotenv;
use poise::{CreateReply, FrameworkError, serenity_prelude as serenity};
use poise::serenity_prelude::{Color, CreateAllowedMentions};
use regex::Regex;
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};

const BOT_COLOR: Color = Color::new(0xfcaaf9);

pub struct Data {
    regex_cache: RegexCache,
    next_reminder: Mutex<Option<i64>>,
    pool: SqlitePool,
} // User data, which is stored and accessible in all command invocations
pub struct RegexCache {
    /// n years, n Months, n weeks, n days, n hours, n minutes, n seconds
    relative_time: Regex,
    /// yyyyMMdd hhmmss
    datetime_ymd: Regex,
    /// ddMMyyyy hhmmss
    datetime_dmy: Regex,
    /// yyyyMMdd
    date_ymd: Regex,
    /// ddmmyyyy
    date_dmy: Regex,
    /// hhmmss
    time: Regex,
    /// n minutes
    relative_minutes: Regex,
    /// epoch time or discord timestamp
    unix_timestamp: Regex,
    /// \+ or - followed by hhmm
    utc_offset: Regex,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Arc<Data>, Error>;
pub type FrameworkContext<'a> = poise::FrameworkContext<'a, Arc<Data>, Error>;
pub type Command = poise::Command<Arc<Data>, Error>;

#[tokio::main]
async fn main() {
    dotenv().expect(".env file not found");
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let database_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS;

    let regex_cache = RegexCache {
        relative_time: Regex::new(r"^(?:(\d+)[yY](?:[a-zA-Z]+)?)?(?:(\d+)(?:M|mo)(?:[a-zA-Z]+)?)?(?:(\d+)[wW](?:[a-zA-Z]+)?)?(?:(\d+)[dD](?:[a-zA-Z]+)?)?(?:(\d+)[hH](?:[a-zA-Z]+)?)?(?:(\d+)m(?:[a-zA-Z]+)?)?(?:(\d+)[sS](?:[a-zA-Z]+)?)?$").unwrap(),
        datetime_ymd: Regex::new(r"^(2\d{3})[/\-.](1[012]|0?[1-9])[/\-.](3[01]|[12]\d|0?[1-9]) (2[0123]|1\d|0?\d)[:.]([12345]\d|0?\d)(?:[:.]([12345]\d|0?\d))?$").unwrap(),
        datetime_dmy: Regex::new(r"^(3[01]|[12]\d|0?[1-9])[/\-.](1[012]|0?[1-9])(?:[/\-.](2\d{3}|\d{2}))? (2[0123]|1\d|0?\d)[:.]([12345]\d|0?\d)(?:[:.]([12345]\d|0?\d))?$").unwrap(),
        date_ymd: Regex::new(r"^(2\d{3})[/\-.](1[012]|0?[1-9])[/\-.](3[01]|[12]\d|0?[1-9])$").unwrap(),
        date_dmy: Regex::new(r"^(3[01]|[12]\d|0?[1-9])[/\-.](1[012]|0?[1-9])(?:[/\-.](2\d{3}|\d{2}))?$").unwrap(),
        time: Regex::new(r"^(2[0123]|1\d|0?\d)[:.]([12345]\d|0?\d)(?:[:.]([12345]\d|0?\d))?$").unwrap(),
        relative_minutes: Regex::new(r"^(\d{1,6})$").unwrap(),
        unix_timestamp: Regex::new(r"^(?:<.:)?(\d{10,16})(?:(?::.)?>)?$").unwrap(),
        utc_offset: Regex::new(r"(?i)^(?:UTC|GMT)?(-(?:1[0-2]|0?\d?)|\+?(?:1[0-4]|0?\d?)):?(00|30|45)?$").unwrap(),
    };
    let pool = SqlitePool::connect(&database_url).await.unwrap();
    let data = Arc::new(Data { regex_cache, next_reminder: Mutex::new(None), pool });

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("h!".into()),
                ..Default::default()
            },
            commands: commands::commands(),
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                let ctx_clone = ctx.clone();
                let data_clone = data.clone();
                tokio::spawn(async move { task_handler(ctx_clone, data_clone).await });
                Ok(data)
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents).framework(framework).await;
    client.unwrap().start().await.unwrap();
}

async fn on_error(error: FrameworkError<'_, Arc<Data>, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        FrameworkError::Command { error, ctx, .. } => {
            let error = error.to_string();
            eprintln!("An error occurred in a command: {error}");

            let mentions = CreateAllowedMentions::new()
                .everyone(false)
                .all_roles(false)
                .all_users(false);

            let _ = ctx.send(
                CreateReply::default()
                    .content(error)
                    .allowed_mentions(mentions)
                    .ephemeral(true),
            )
            .await;
            ()
        }
        FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            let error = error.map_or("yup".to_string(), |e| e.to_string());
            eprintln!("Command check failed: {error}");

            let mentions = CreateAllowedMentions::new()
                .everyone(false)
                .all_roles(false)
                .all_users(false);

            let _ = ctx.send(
                CreateReply::default()
                    .content(error)
                    .allowed_mentions(mentions)
                    .ephemeral(true),
            )
            .await;
        }
        _ => ()
    }
}