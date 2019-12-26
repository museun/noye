use crate::bot::prelude::*;

#[cfg(test)]
#[macro_export]
macro_rules! ensure_api_key_for {
    ($what:expr) => {
        config::load_env().unwrap();
        assert!(!get_api_key($what).unwrap().is_empty());
    };
}

/// Generate a Registry
///
/// This is used to bind handlers to their functions
/// ```rust,no-run
/// registry!(
///     // kind     // type        // method
///     listener => Command::Nick, reclaim;
///     command  => "!join",       join;
///     passive  => LINK_REGEX,    hear_instagram;
/// );
/// ```

#[macro_export]
macro_rules! registry {
    ($name:expr => { $($handler:tt => $arg:expr, $func:ident );* $(;)?} ) => {
        #[derive(Default)]
        pub(super) struct Registry;

        impl super::Registry for Registry {
            fn name(&self) -> &'static str {
                $name
            }
            fn register(&self, dispatcher: &mut Dispatcher) {
                $(dispatcher.$handler($arg, $func);)*
            }
        }
    };
}

macro_rules! import_modules {
    ($($ident:tt);* $(;)?) => {
        $(pub(super) mod $ident;)*
        fn available_modules() -> Vec<Module> {
            vec![$(Module::new::<$ident::Registry>(),)*]
        }
    };
}

trait Registry {
    fn register(&self, dispatcher: &mut Dispatcher);
    fn name(&self) -> &'static str;
}

// TODO keep track of whats added
struct Module {
    #[allow(dead_code)]
    name: &'static str,
    registry: Box<dyn Registry>,
}

impl Module {
    fn new<R>() -> Self
    where
        R: Registry + Default + 'static,
    {
        let module = R::default();
        Self {
            name: module.name(),
            registry: Box::new(module),
        }
    }
}

import_modules!(
    builtin;
    youtube;
    link_size;
    vimeo;
    gdrive;
    instagram;
);

pub fn load_modules(_config: &Config, dispatcher: &mut Dispatcher) {
    for module in available_modules() {
        // TODO check for disabled modules
        // if disabled_modules.check(Channel("*"), Module(module.name)) {
        //     log::info!("disabling module '{}' globally", module.name);
        //     continue;
        // }
        module.registry.register(dispatcher);
    }
}
