use std::{io, os::unix::prelude::CommandExt, process::Command, fs};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use pop_launcher_toolkit::{
    launcher::{PluginResponse, PluginSearchResult},
    plugin_trait::{
        async_trait,
        tracing::{debug, error},
        PluginExt,
    },
};
use sqlx::{Connection, FromRow, SqliteConnection};


// a temp file to backup chrome history database file
static TMP_PATH: &str = "/tmp/h";

pub struct ChromeHistorys {
    historys: Vec<HistoryEntry>,
    fuzzy_matcher: SkimMatcherV2
}

impl ChromeHistorys {
    async fn new() -> Self {
        let historys = ChromeHistorys::load_all_hisotry().await.unwrap_or(vec![]);
        ChromeHistorys {
            historys,
            fuzzy_matcher: SkimMatcherV2::default().ignore_case().use_cache(true),
        }
    }

    // load all history from chrome's hisotry database
    async fn load_all_hisotry() -> Result<Vec<HistoryEntry>, sqlx::Error> {
        let mut home = dirs::home_dir().expect("$HOME not found");
        home.push(".config/google-chrome/Default/History");

        fs::copy(home.as_path(), TMP_PATH).expect("error: copy chrome's history to {TMP_PATH}" );

        // get a connect to google chrome's history database
        let mut conn = SqliteConnection::connect(&format!("sqlite:{}", TMP_PATH)).await?;

        sqlx::query_as::<_, HistoryEntry>(
            "select title, url from urls order by last_visit_time desc",
        )
        .fetch_all(&mut conn)
        .await
    }

    fn sort_match(&mut self, query: &str){
        self.historys.sort_by(|a, b| {
            let score_a = self.fuzzy_matcher.fuzzy_match(&a.title, query).unwrap_or(-1);
            let score_b = self.fuzzy_matcher.fuzzy_match(&b.title, query).unwrap_or(-1);

            score_b.cmp(&score_a)
        })

    }
}

#[async_trait]
impl PluginExt for ChromeHistorys {
    fn name(&self) -> &str {
        "ch"
    }

    async fn search(&mut self, query: &str) {
        match query.split_once(' ') {
            Some(("ch", query)) => {
                self.sort_match(query);
                for (id, entry) in self.historys.iter().enumerate().take(8) {
                    self.respond_with(PluginResponse::Append(PluginSearchResult {
                        id: id as u32,
                        name: entry.title.clone(),
                        description: entry.url.clone(),
                        keywords: None,
                        icon: None,
                        exec: None,
                        window: None,
                    }))
                    .await;
                }
                self.respond_with(PluginResponse::Finished).await;
            }
            _ => {
                self.respond_with(PluginResponse::Finished).await;
            }
        }
    }

    async fn activate(&mut self, id: u32) {
        self.respond_with(PluginResponse::Close).await;
        match self.historys.get(id as usize) {
            Some(history) => {
                history.exec();
                std::process::exit(0);
            }
            None => {
                error!("entry not found at index {id}");
            }
        }
    }
}

#[derive(Debug, FromRow)]
pub struct HistoryEntry {
    title: String,
    url: String,
}

impl HistoryEntry {
    // open url in chrome
    fn exec(&self) -> io::Error {
        let mut cmd = Command::new("xdg-open");
        cmd.arg(&self.url);

        debug!("excute: {:?}", cmd);
        cmd.exec()
    }
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let mut plugin = ChromeHistorys::new().await;
    plugin.run().await;
}

#[cfg(test)]
mod test {
    use sqlx::{Connection, SqliteConnection};

    use crate::HistoryEntry;
    #[tokio::test]
    async fn connect_sqlite() -> Result<(), anyhow::Error> {
        let mut conn = SqliteConnection::connect("sqlite:/tmp/h").await?;
        let historyEntrys = sqlx::query_as::<_, HistoryEntry>("select title, url from urls")
            .fetch_all(&mut conn)
            .await?;
        for hentry in historyEntrys {
           println!("{}, {}", hentry.title, hentry.url);
        }
        Ok(())
    }
}
