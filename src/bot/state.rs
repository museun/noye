use crate::{util::type_name, CachedConfig, Config};

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

#[derive(Default, Debug)]
pub struct State(HashMap<TypeId, Box<dyn Any + Send + Sync>>);

impl State {
    pub fn insert<T: 'static + Send + Sync>(&mut self, item: T) -> bool {
        self.0.insert(TypeId::of::<T>(), Box::new(item)).is_none()
    }

    pub fn expect_insert<T: 'static + Send + Sync>(&mut self, item: T) -> anyhow::Result<()> {
        if self.0.insert(TypeId::of::<T>(), Box::new(item)).is_some() {
            anyhow::bail!("'{}' already existed in state", type_name::<T>())
        }

        Ok(())
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|item| item.downcast_ref::<T>())
    }

    pub fn get_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut T> {
        self.0
            .get_mut(&TypeId::of::<T>())
            .and_then(|item| item.downcast_mut::<T>())
    }

    pub fn expect_get<T: 'static + Send + Sync>(&self) -> anyhow::Result<&T> {
        self.get()
            .ok_or_else(|| anyhow::anyhow!("cannot get: {}", type_name::<T>()))
    }

    pub fn expect_get_mut<T: 'static + Send + Sync>(&mut self) -> anyhow::Result<&mut T> {
        self.get_mut()
            .ok_or_else(|| anyhow::anyhow!("cannot get mut: {}", type_name::<T>()))
    }

    pub async fn config(&mut self) -> anyhow::Result<&Config> {
        let config = self
            .get_mut::<CachedConfig>()
            .ok_or_else(|| anyhow::anyhow!("cannot get config"))?;
        config.get().await
    }
}
