use anyhow::Context as _;
use std::sync::Arc;
use template::TemplateStore;
use tokio::sync::Mutex;

pub type Resolver = Arc<Mutex<template::Resolver<Box<dyn TemplateStore + Send>>>>;

pub fn new<S: TemplateStore + Send + 'static>(store: S) -> anyhow::Result<Resolver> {
    template::Resolver::new(Box::new(store) as Box<dyn TemplateStore + Send>)
        .with_context(|| "cannot create template resolver")
        .map(Mutex::new)
        .map(Arc::new)
}
