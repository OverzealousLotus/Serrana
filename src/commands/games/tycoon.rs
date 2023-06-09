// Tycoon

// <=== Standard Library ===>
use std::collections::BTreeMap;
use std::env;

// <=== Tokio ===>
use tokio::fs::{write, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::time::{sleep, Duration};

// <=== Event Tracking ===>
use tracing::{error, info};

// <=== Database Parsing ===>
use serde::{Deserialize, Serialize};

use crate::serenity::MessageBuilder;
use crate::{Context, Error};

// <===== Constants =====>
const MAIN_MENU: &str = "What would you like to do?:
Procure: Procure credits by working!
Exit | Quit | Abort: Exit Tycoon!
Todo!: Unimplemented!";
const HELP_MESSAGE: &str = "Options:
Credits: Todo!
Cuties: Currency used to gamble in games!";

// <===== Functions =====>
async fn intake(prompt: &str, context: Context<'_>) -> String {
    let _ = context.say(prompt).await;
    if let Some(answer) = context
        .author()
        .await_reply(context)
        .timeout(Duration::from_secs(10))
        .await
    {
        answer.content.to_string()
    } else {
        error!("An error occurred trying to fetch intake!");
        String::from("Noop")
    }
}

async fn wait(time: f64) {
    sleep(Duration::from_secs_f64(time)).await;
}

async fn speak(command: &str, response: &str, context: Context<'_>) {
    if let Err(reason) = context.say(response).await {
        error!("An error occurred speaking!: {}", reason)
    } else {
        info!("Speak was invoked for {}!", command);
    }
}

// <=== Fetch contents of Database ===>
async fn readable(database_path: String) -> String {
    // Prepare to read Database.
    let file = OpenOptions::new()
        .read(true)
        .open(database_path)
        .await
        .expect("Error opening file!");
    let mut reader = BufReader::new(file);
    let mut contents = String::new();

    // Read Database and store.
    reader
        .read_to_string(&mut contents)
        .await
        .expect("Error reading database!");

    contents
}

async fn produce(
    database_path: String,
    mut amount: usize,
    user: String,
    context: Context<'_>,
) -> Result<(), Error> {
    let mut database: Database = toml::from_str(&readable(database_path.clone()).await)
        .expect("Error parsing database to struct!");

    // Increase user credits by specified amount.
    speak("Tycoon: PRODUCE", "Procuring credits...", context).await;
    if amount > 50 {
        wait(1.0).await;
        amount -= amount;
        database
            .players
            .entry(user.clone())
            .and_modify(|player| player.credits += amount);
    }

    while amount != 0 {
        wait(0.5).await;
        amount -= 1;
        database
            .players
            .entry(user.clone())
            .and_modify(|player| player.credits += 1);
    }
    let response = MessageBuilder::new()
        .push("Produced credit! New count: ")
        .push_bold_safe(database.players.get(&user).unwrap().credits)
        .build();
    context.say(response).await?;

    // Save changes to database.
    let new_db = toml::to_string(&database).expect("Error parsing database to TOML!");
    write(database_path, new_db)
        .await
        .expect("Error rewriting database!");

    Ok(())
}

async fn game_loop(context: Context<'_>) -> Result<(), Error> {
    // Make sure to include a path to your database in an .env file.
    dotenvy::dotenv().expect("Error reading environment!");
    info!(
        "Tycoon started by {} in channel: {}!",
        context.author().name,
        context.channel_id()
    );
    let current_user = context.author().name.to_lowercase();
    let database_path: String =
        env::var("DATABASE_PATH").expect("Error fetching path to Database!");

    'main_loop: loop {
        let decision = intake(MAIN_MENU, context).await.trim().to_uppercase();

        match decision.as_str() {
            "PRODUCE" | "PROCURE" => {
                let time = intake("How long?: ", context).await.parse::<usize>()?;
                produce(database_path.clone(), time, current_user.clone(), context).await?;
            }

            "QUIT" | "EXIT" | "ABORT" => {
                speak("EXIT", "Exited tycoon!", context).await;
                break 'main_loop;
            }

            "NOOP" => {
                speak("NOOP", "No response, aborting!", context).await;
                error!("{} took too long to respond!", context.author().name);
                break 'main_loop;
            }

            _ => {
                speak("INVALID", "Invalid response, retrying!", context).await;
            }
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn register_player(context: Context<'_>) -> Result<(), Error> {
    let database_path = env::var("DATABASE_PATH").expect("Error reading path to database!");
    let mut database = Database::default();
    let new_user = intake("Enter new user: ", context)
        .await
        .trim()
        .to_lowercase();

    database.players.insert(
        new_user.clone(),
        Player {
            username: new_user.to_uppercase(),
            credits: 0,
            cuties: 0,
        },
    );
    let parsed_user = toml::to_string(&database).expect("Error parsing new user to TOML!");

    let mut file = OpenOptions::new()
        .append(true)
        .read(true)
        .open(database_path)
        .await
        .expect("Failed to fetch path to database!");
    let mut reader = BufReader::new(&mut file);
    let mut contents = String::new();

    reader
        .read_to_string(&mut contents)
        .await
        .expect("Error reading database!");

    if contents.contains(&new_user) {
        error!("{} already exists in database!", &new_user);
    } else {
        info!("Registering {} into database!", &new_user);
        if let Err(reason) = file.write(parsed_user.as_bytes()).await {
            error!("{:?}", reason);
        }
    }
    Ok(())
}

// <===== Structs =====>
#[derive(Serialize, Deserialize, Default, Debug)]
struct Player {
    username: String,
    credits: usize,
    cuties: usize,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct Database {
    players: BTreeMap<String, Player>,
}

// <===== Command =====>
#[poise::command(slash_command, prefix_command)]
pub(crate) async fn tycoon(
    context: Context<'_>,
    #[description = "TBA"] args: String,
) -> Result<(), Error> {
    let argument = args.trim();
    if argument.to_uppercase() != "HELP" {
        game_loop(context).await?;
    } else {
        speak("HELP", HELP_MESSAGE, context).await;
    }
    Ok(())
}
