use std::collections::HashMap;

use excel_model::Doc;

use crate::{Function, date, logic, lookup, math, text};

/// The functions a formula can call.
#[derive(Default)]
pub struct FunctionRegistry {
    functions: HashMap<&'static str, Box<dyn Function + Send + Sync>>,
}

impl FunctionRegistry {
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        registry.register(math::sum::Sum);
        registry.register(math::average::Average);
        registry.register(math::count::Count);
        registry.register(math::min::Min);
        registry.register(math::max::Max);
        registry.register(math::abs::Abs);
        registry.register(math::int::Int);
        registry.register(math::mod_fn::Mod);
        registry.register(math::power::Power);
        registry.register(math::sqrt::Sqrt);
        registry.register(math::product::Product);
        registry.register(math::round::Round);
        registry.register(math::roundup::RoundUp);
        registry.register(math::rounddown::RoundDown);
        registry.register(math::counta::CountA);
        registry.register(math::countblank::CountBlank);
        registry.register(math::countif::CountIf);
        registry.register(math::countifs::CountIfs);
        registry.register(math::sumif::SumIf);
        registry.register(math::sumifs::SumIfs);
        registry.register(math::averageif::AverageIf);
        registry.register(math::ifs_range::AverageIfs);
        registry.register(math::ifs_range::MaxIfs);
        registry.register(math::ifs_range::MinIfs);
        registry.register(math::sumproduct::SumProduct);
        registry.register(math::multiple::Ceiling);
        registry.register(math::multiple::Floor);
        registry.register(math::stats::Median);
        registry.register(math::stats::Large);
        registry.register(math::stats::Small);
        registry.register(math::stats::Rank);
        registry.register(math::stats::StDev);
        registry.register(logic::if_fn::If);
        registry.register(logic::ifs::Ifs);
        registry.register(logic::iferror::IfError);
        registry.register(logic::ifna::IfNa);
        registry.register(logic::switch::Switch);
        registry.register(logic::isna::IsNa);
        registry.register(logic::and::And);
        registry.register(logic::or::Or);
        registry.register(logic::not::Not);
        registry.register(logic::isblank::IsBlank);
        registry.register(logic::isnumber::IsNumber);
        registry.register(logic::istext::IsText);
        registry.register(logic::iserror::IsError);
        registry.register(text::len::Len);
        registry.register(text::left::Left);
        registry.register(text::right::Right);
        registry.register(text::mid::Mid);
        registry.register(text::upper::Upper);
        registry.register(text::lower::Lower);
        registry.register(text::trim::Trim);
        registry.register(text::concat::Concat);
        registry.register(text::textjoin::TextJoin);
        registry.register(text::find::Find);
        registry.register(text::substitute::Substitute);
        registry.register(text::text_fn::Text);
        registry.register(text::search::Search);
        registry.register(text::replace::Replace);
        registry.register(text::rept::Rept);
        registry.register(text::text_split::TextBefore);
        registry.register(text::text_split::TextAfter);
        registry.register(text::value::Value);
        registry.register(lookup::vlookup::VLookup);
        registry.register(lookup::hlookup::HLookup);
        registry.register(lookup::index::Index);
        registry.register(lookup::match_fn::Match);
        registry.register(lookup::xlookup::XLookup);
        registry.register(lookup::choose::Choose);
        registry.register(date::today::Today);
        registry.register(date::date_fn::Date);
        registry.register(date::year::Year);
        registry.register(date::month::Month);
        registry.register(date::day::Day);
        registry.register(date::datedif::DateDif);
        registry.register(date::edate::EDate);
        registry.register(date::edate::EoMonth);
        registry.register(date::weekday_fn::Weekday);
        registry.register(date::networkdays::NetworkDays);
        registry.register(date::workday::Workday);
        registry.register(date::days::Days);
        registry
    }

    pub fn register(&mut self, function: impl Function + Send + Sync + 'static) {
        self.functions.insert(function.name(), Box::new(function));
    }

    /// Every function's name and help, sorted by name.
    pub fn list(&self) -> Vec<(&'static str, Doc)> {
        let mut list: Vec<_> = self.functions.values().map(|function| (function.name(), function.doc())).collect();
        list.sort_unstable_by_key(|(name, _)| *name);
        list
    }

    /// Finds a function by its uppercase name.
    pub fn get(&self, name: &str) -> Option<&(dyn Function + Send + Sync)> {
        self.functions.get(name).map(|function| &**function)
    }
}
