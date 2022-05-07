use std::collections::HashMap;
use std::io::{BufRead, BufReader, Cursor};
use std::marker::PhantomData;

use binrw::BinReaderExt;
use serde::de::DeserializeOwned;
use unicase::Ascii;

use crate::data::repo::Repository;
use crate::error::LastLegendError;
use crate::simple_task::{format_index_entry_for_console, read_entry_header};
use crate::surpass::page::{PageHeader, RowBufferIter};
use crate::surpass::serde_row::from_row;
use crate::surpass::sheet_info::{Language, SheetInfo};

#[derive(Debug)]
pub struct Collection {
    repo: Repository,
    sheets: HashMap<Ascii<String>, i32>,
}

/// Magic value for the root file that points to all sheets.
const MAGIC_ROOT: &str = "exd/root.exl";

impl Collection {
    pub fn load(repo: Repository) -> Result<Self, LastLegendError> {
        let index = repo
            .get_index_for(MAGIC_ROOT)
            .map_err(|e| e.add_context("Failed to read index for collection"))?;
        let (header, dat_reader) = read_entry_header(&index, MAGIC_ROOT)
            .map_err(|e| e.add_context("Failed to open data reader for collection"))?;
        let reader = header
            .read_content(dat_reader)
            .map_err(|e| LastLegendError::Io("Couldn't open content reader".into(), e))?;

        let mut sheets = HashMap::new();
        for line in BufReader::new(reader).lines() {
            let line = line.map_err(|e| LastLegendError::Io("Failed to read line".into(), e))?;
            let (name, id_str) = line
                .split_once(',')
                .ok_or_else(|| LastLegendError::CollectionSheetLineInvalid(line.clone()))?;
            sheets.insert(
                Ascii::new(name.to_string()),
                id_str
                    .parse()
                    .map_err(|_| LastLegendError::CollectionSheetLineInvalid(line))?,
            );
        }

        Ok(Self { repo, sheets })
    }

    pub fn sheet_iter(&self, name: &str) -> Result<SheetIter, LastLegendError> {
        self.get_sheet_info(name).map(|sheet_info| SheetIter {
            repo: self.repo.clone(),
            sheet_name: name.to_string(),
            sheet_info,
            current_page: 0,
            current_page_iter: None,
        })
    }

    fn get_sheet_info(&self, name: &str) -> Result<SheetInfo, LastLegendError> {
        let name = Ascii::new(name.to_string());
        // Normalize name by getting the value used in the map.
        let (name, _id) = self
            .sheets
            .get_key_value(&name)
            .ok_or_else(|| LastLegendError::SheetNameInvalid(name.into_inner()))?;
        let name = name.clone().into_inner();

        let file_name = format!("exd/{0}.exh", name);
        let index = self
            .repo
            .get_index_for(&file_name)
            .map_err(|e| e.add_context("Failed to read index for collection"))?;

        log::debug!(
            "Loading sheet info {}",
            format_index_entry_for_console(
                &self.repo.repo_path(),
                &index,
                index.get_entry(&file_name)?,
                &file_name
            )
        );

        let (header, dat_reader) = read_entry_header(&index, &file_name)
            .map_err(|e| e.add_context("Failed to open data reader for collection"))?;
        let content = header
            .read_content_to_vec(dat_reader)
            .map_err(|e| LastLegendError::Io("Failed to read dat content".into(), e))?;

        Cursor::new(content)
            .read_be::<SheetInfo>()
            .map_err(|e| LastLegendError::BinRW("Failed to read sheet header".into(), e))
    }
}

pub struct SheetIter {
    repo: Repository,
    sheet_name: String,
    sheet_info: SheetInfo,
    current_page: usize,
    current_page_iter: Option<RowBufferIter<Cursor<Vec<u8>>>>,
}

impl SheetIter {
    pub fn sheet_info(&self) -> &SheetInfo {
        &self.sheet_info
    }

    pub fn deserialize_rows<T: DeserializeOwned>(self) -> DeSheetIter<T> {
        DeSheetIter {
            sheet_iter: self,
            _marker: PhantomData,
        }
    }

    fn load_page_iter(
        &mut self,
        page_start: u32,
    ) -> Result<RowBufferIter<Cursor<Vec<u8>>>, LastLegendError> {
        let language = self
            .sheet_info
            .languages
            .iter()
            .find(|&&l| l == Language::None || l == Language::English)
            .unwrap_or_else(|| {
                panic!(
                    "Language must be None or English, have {:?}",
                    self.sheet_info.languages
                )
            });
        let file_name = language.get_sheet_name(&self.sheet_name, page_start);
        let index = self
            .repo
            .get_index_for(&file_name)
            .map_err(|e| e.add_context("Failed to read sheet page"))?;

        log::debug!(
            "Loading sheet page {}",
            format_index_entry_for_console(
                &self.repo.repo_path(),
                &index,
                index.get_entry(&file_name)?,
                &file_name
            )
        );

        let (header, dat_reader) = read_entry_header(&index, &file_name)
            .map_err(|e| e.add_context("Failed to open data reader for sheet page"))?;
        let content = header
            .read_content_to_vec(dat_reader)
            .map_err(|e| LastLegendError::Io("Failed to read dat content".into(), e))?;

        let mut cursor = Cursor::new(content);
        let page_header = cursor
            .read_be::<PageHeader>()
            .map_err(|e| LastLegendError::BinRW("Failed to read page header".into(), e))?;
        Ok(page_header.row_buffer_iter(cursor, &self.sheet_info))
    }
}

impl Iterator for SheetIter {
    type Item = Result<Vec<u8>, LastLegendError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.current_page_iter {
                Some(iter) => {
                    let item = iter.next();
                    if item.is_some() {
                        return item;
                    }
                    self.current_page += 1;
                    self.current_page_iter = None;
                }
                None => {
                    let page_start = match self.sheet_info.page_ranges.get(self.current_page) {
                        Some(range) => range.start,
                        None => return None,
                    };
                    match self.load_page_iter(page_start) {
                        Ok(iter) => self.current_page_iter = Some(iter),
                        Err(e) => return Some(Err(e)),
                    }
                }
            }
        }
    }
}

pub struct DeSheetIter<T> {
    sheet_iter: SheetIter,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned> Iterator for DeSheetIter<T> {
    type Item = Result<T, LastLegendError>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.sheet_iter.next();
        next.map(|r| {
            r.and_then(|row| {
                from_row(
                    &self.sheet_iter.sheet_info.columns,
                    self.sheet_iter.sheet_info.fixed_row_size as u64,
                    row,
                )
            })
        })
    }
}
