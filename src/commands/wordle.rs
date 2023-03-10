use std::sync::Arc;
use std::time::Duration;

use crate::utils::tokens::add_tokens;
use crate::utils::wordle::{self};
use rand::seq::IteratorRandom;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{CommandError, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;

#[command]
pub async fn wordle(ctx: &Context, msg: &Message) -> CommandResult {
    let solution = choose_word().await;
    let mut game_state = wordle::Game::new(&solution);

    while !game_state.is_game_over() {
        let guess_left_mssg = format!("Input guess {} of 6", 6 - game_state.guesses_left() + 1);
        msg.reply(&ctx, guess_left_mssg).await?;

        if let Some(response) = collect_response(ctx, msg).await? {
            match game_state.guess(&response.content).await {
                Ok(data) => {
                    response.reply(ctx, format!("{}", data)).await?;
                }
                Err(_) => {
                    response.reply(ctx, "Invalid guess!").await?;
                    continue;
                }
            }
        } else {
            let timeout_mssg = MessageBuilder::new()
                .push("Wordle game timed out!  Word was ")
                .push_bold(solution)
                .build();
            msg.reply(ctx, timeout_mssg).await?;
            return Ok(());
        }
    }

    if game_state.game_won() {
        let tokens_won = (game_state.guesses_left() * 5) + 5;

        let game_won_mssg = format!(
            "You won on try {}\nYou win {} tokens!",
            6 - game_state.guesses_left(),
            tokens_won
        );
        msg.reply(ctx, game_won_mssg).await?;
        add_tokens(ctx, msg.author.id, tokens_won as u64).await?;
    } else {
        let lose_mssg = MessageBuilder::new()
            .push("You lost!  The correct word was ")
            .push_bold(solution)
            .build();
        msg.reply(ctx, lose_mssg).await?;
    }

    Ok(())
}

type ResponseResult = Result<Option<Arc<Message>>, CommandError>;

async fn collect_response(ctx: &Context, msg: &Message) -> ResponseResult {
    if let Some(answer) = &msg
        .author
        .await_reply(ctx)
        .timeout(Duration::from_secs(60))
        .await
    {
        return Ok(Some(answer.clone()));
    }
    Ok(None)
}

async fn choose_word() -> String {
    let wordlist = tokio::fs::read_to_string("wordlist.txt")
        .await
        .expect("Failed to load word list");
    let mut rng = rand::thread_rng();
    let solution = wordlist.lines().choose(&mut rng).unwrap();
    solution.to_string()
}
