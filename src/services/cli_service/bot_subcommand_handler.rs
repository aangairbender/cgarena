use std::rc::Rc;

use colored::Colorize;
use tabled::Tabled;

use crate::models::Bot;
use crate::services::bot_service::BotService;

pub struct BotSubCommandHandler {
    bot_service: Rc<BotService>,
}

impl BotSubCommandHandler {
    pub fn new(bot_service: Rc<BotService>) -> Self {
        Self { bot_service }
    }

    pub fn cmd_list(&self) {
        let mut bots = self.bot_service.list_bots().collect::<Vec<Bot>>();
        bots.sort_by(|a, b| a.rating().partial_cmp(&b.rating()).unwrap().reverse());
        let table = tabled::Table::new(bots.iter().map(BotView::from));
        println!("{}", table);
    }

    pub fn cmd_bot_remove(&self, name: String) {
        self.bot_service.remove_bot(&name);
        println!("     {} bot '{}'", "Removed".bright_red(), name);
    }

    pub fn cmd_bot_add(&self, name: String, file: String, language: Option<String>) {
        let bot = self
            .bot_service
            .add_bot(name, file, language)
            .expect("bot should be added successfully");
        println!(
            "     {} bot '{}' written in '{:?}'",
            "Added".bright_green(),
            bot.name,
            bot.language_name
        );
    }
}

#[derive(Tabled)]
struct BotView<'a> {
    name: &'a str,
    language_name: &'a str,
    matches: u32,
    rating: f64,
}

impl<'a> From<&'a Bot> for BotView<'a> {
    fn from(bot: &'a Bot) -> Self {
        Self {
            name: &bot.name,
            language_name: &bot.language_name,
            matches: bot.completed_matches,
            rating: bot.rating(),
        }
    }
}
