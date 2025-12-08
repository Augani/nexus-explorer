use nucleo::{Config, Injector, Matcher, Nucleo};
use std::path::PathBuf;
use std::sync::Arc;


pub struct SearchEngine {
    nucleo: Nucleo<PathBuf>,
    pattern: String,
    active: bool,
}


#[derive(Debug, Clone)]
pub struct SearchSnapshot {
    pub matches: Vec<MatchedItem>,
    pub pattern: String,
    pub total_items: usize,
}


#[derive(Debug, Clone)]
pub struct MatchedItem {
    pub path: PathBuf,
    pub score: u32,
    pub positions: Vec<usize>,
}

impl SearchEngine {

    pub fn new() -> Self {
        let config = Config::DEFAULT.match_paths();
        let nucleo = Nucleo::new(config, Arc::new(|| {}), None, 1);

        Self {
            nucleo,
            pattern: String::new(),
            active: false,
        }
    }


    pub fn injector(&self) -> Injector<PathBuf> {
        self.nucleo.injector()
    }


    pub fn inject(&self, path: PathBuf) {
        let injector = self.nucleo.injector();
        let path_string = path.to_string_lossy().to_string();
        injector.push(path, move |_p, cols| {
            cols[0] = path_string.as_str().into();
        });
    }


    pub fn set_pattern(&mut self, pattern: &str) {
        self.pattern = pattern.to_string();
        self.active = !pattern.is_empty();

        self.nucleo.pattern.reparse(
            0,
            pattern,
            nucleo::pattern::CaseMatching::Smart,
            nucleo::pattern::Normalization::Smart,
            false,
        );
    }


    pub fn pattern(&self) -> &str {
        &self.pattern
    }


    pub fn is_active(&self) -> bool {
        self.active
    }


    pub fn snapshot(&mut self) -> SearchSnapshot {
        self.nucleo.tick(10);

        let snapshot = self.nucleo.snapshot();
        let total_items = snapshot.item_count() as usize;

        let mut matcher = Matcher::new(Config::DEFAULT);

        let matches: Vec<MatchedItem> = snapshot
            .matched_items(0..snapshot.matched_item_count().min(1000))
            .map(|item| {
                let path = item.data.clone();

                let mut indices: Vec<u32> = Vec::new();
                let pattern = snapshot.pattern().column_pattern(0);
                pattern.indices(
                    item.matcher_columns[0].slice(..),
                    &mut matcher,
                    &mut indices,
                );
                let positions: Vec<usize> = indices.iter().map(|&i| i as usize).collect();

                MatchedItem {
                    path,
                    score: 0,
                    positions,
                }
            })
            .collect();

        SearchSnapshot {
            matches,
            pattern: self.pattern.clone(),
            total_items,
        }
    }


    pub fn clear(&mut self) {
        self.pattern.clear();
        self.active = false;

        let config = Config::DEFAULT.match_paths();
        self.nucleo = Nucleo::new(config, Arc::new(|| {}), None, 1);
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchSnapshot {

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }


    pub fn len(&self) -> usize {
        self.matches.len()
    }
}

impl MatchedItem {

    pub fn is_match_position(&self, pos: usize) -> bool {
        self.positions.contains(&pos)
    }
}

#[cfg(test)]
#[path = "search_engine_tests.rs"]
mod tests;
