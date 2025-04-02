use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use flagset::{flags, FlagSet};
use log::{debug, error};
use notify_rust::Notification;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::Error;

fn report_unknown_kwargs(
    context: &str,
    known: &[&str],
    kwargs: &HashMap<String, String>,
) -> Option<Error> {
    if kwargs.keys().any(|k| !known.contains(&k.as_str())) {
        Some(Error::EffectError(format!(
            "Invalid keyword argument(s) passed to `{}`: {:?}, valid keywords are: {:?}",
            context,
            kwargs
                .keys()
                .filter(|k| !known.contains(&k.as_str()))
                .collect::<Vec<_>>(),
            known,
        )))
    } else {
        None
    }
}

flags! {
    #[derive(Default)]
    pub enum EffectOptions: u32 {
        #[default]
        Defaults = 0,

        SilentTest = 1,
    }
}

pub trait EffectOptionsExt {
    fn is_silent_test(&self) -> bool;
}

impl EffectOptionsExt for FlagSet<EffectOptions> {
    fn is_silent_test(&self) -> bool {
        self.contains(EffectOptions::SilentTest)
    }
}

pub type EffectArgs<'a> = &'a [String];
pub type EffectKwArgs<'a> = &'a HashMap<String, String>;
pub type EffectSignature = fn(EffectArgs, EffectKwArgs, FlagSet<EffectOptions>) -> Option<Error>;

#[derive(Debug, Clone)]
pub struct EffectInvocation {
    name: String,
    args: Vec<String>,
    kwargs: HashMap<String, String>,
}

impl Hash for EffectInvocation {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.name.hash(hasher);

        for (n, arg) in self.args.iter().enumerate() {
            n.hash(hasher);
            arg.hash(hasher);
        }

        let mut keys = self.kwargs.keys().collect::<Vec<_>>();
        keys.sort();

        for key in keys {
            key.hash(hasher);
            self.kwargs
                .get(key)
                .expect("key still exists in map")
                .hash(hasher);
        }
    }
}

impl EffectInvocation {
    pub fn new(
        name: impl Into<String>,
        args: Vec<String>,
        kwargs: HashMap<String, String>,
    ) -> Self {
        EffectInvocation {
            name: name.into(),
            args,
            kwargs,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn args(&self) -> &Vec<String> {
        &self.args
    }

    pub fn kwargs(&self) -> &HashMap<String, String> {
        &self.kwargs
    }
}

pub async fn default_effects_runner_task(
    mut effects_receiver: UnboundedReceiver<EffectInvocation>,
) {
    loop {
        match effects_receiver.recv().await {
            Some(invocation) => {
                let effect_fn = match invocation.name() {
                    "print" => Some(print as EffectSignature),
                    "notify" => Some(notify as EffectSignature),
                    _ => None,
                };

                debug!(
                    "effect::default_effects_runner_task: invoking `{}` (args: {:?}, kwargs: {:?})",
                    invocation.name(),
                    invocation.args(),
                    invocation.kwargs()
                );

                match effect_fn {
                    Some(f) => {
                        if let Some(e) = f(
                            invocation.args(),
                            invocation.kwargs(),
                            EffectOptions::default().into(),
                        ) {
                            error!(
                                "effect::default_effects_runner_task: \
                                error invoking effect `{}`: {e} (args: {:?}, kwargs: {:?})",
                                invocation.name(),
                                invocation.args(),
                                invocation.kwargs(),
                            );
                        }
                    }
                    None => error!(
                        "effect::default_effects_runner_task: unknown effect `{}`",
                        invocation.name(),
                    ),
                }
            }
            None => return,
        }
    }
}

pub fn print(
    args: EffectArgs,
    kwargs: EffectKwArgs,
    opts: FlagSet<EffectOptions>,
) -> Option<Error> {
    macro_rules! maybe_print {
        ($($stuff:expr),+) => {
            if !opts.is_silent_test() {
                print!($($stuff),+);
            }
        };
    }

    for arg in args.iter().take(args.len() - 1) {
        maybe_print!("{arg} ");
    }

    maybe_print!("{}", args[args.len() - 1]);

    match kwargs.get("end") {
        Some(str) => maybe_print!("{str}"),
        None => maybe_print!("\n"),
    }

    report_unknown_kwargs("print", &["end"], kwargs)
}

pub fn notify(
    args: EffectArgs,
    kwargs: EffectKwArgs,
    opts: FlagSet<EffectOptions>,
) -> Option<Error> {
    let args_joined = args.to_vec().join(" ");
    let mut notification = Notification::new();

    notification.body(match kwargs.get("body") {
        Some(text) => text,
        None => &args_joined,
    });

    if let Some(appname) = kwargs.get("appname") {
        notification.appname(appname);
    }

    if let Some(title) = kwargs.get("title") {
        notification.summary(title);
    }

    if let Some(icon) = kwargs.get("icon") {
        notification.icon(icon);
    }

    if let Some(sound) = kwargs.get("sound") {
        notification.sound_name(sound);
    }

    let send_error = if !opts.is_silent_test() {
        match notification.show() {
            Err(e) => Some(format!("{e}")),
            _ => None,
        }
    } else {
        None
    };

    let kw_error = report_unknown_kwargs(
        "notify",
        &["body", "appname", "title", "icon", "sound"],
        kwargs,
    )
    .map(|e| match e {
        Error::EffectError(text) => text,
        _ => panic!("unreachable"),
    });

    match (send_error, kw_error) {
        (Some(s1), Some(s2)) => Some(Error::EffectError(format!("{s1}\n{s2}"))),
        (Some(s1), None) => Some(Error::EffectError(s1.to_string())),
        (None, Some(s2)) => Some(Error::EffectError(s2.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! map {
        ($($key:expr => $val:expr),+) => {
            {
                let mut x = HashMap::<String, String>::new();
                $(
                    x.insert($key.to_string(), $val.to_string());
                )+
                x
            }
        };
    }

    #[test]
    fn test_report_unknown_kwargs() {
        assert!(report_unknown_kwargs("test", &["a", "b", "c"], &HashMap::new()).is_none());

        assert!(report_unknown_kwargs(
            "test",
            &["a", "b", "c"],
            &map!("a" => 1, "b" => 2, "c" => 3)
        )
        .is_none());

        assert!(report_unknown_kwargs("test", &[], &map!("a" => 1)).is_some());
        assert!(report_unknown_kwargs("test", &["a", "b", "c"], &map!["d" => 1]).is_some());
    }

    #[test]
    fn test_print() {
        assert!(print(
            &["hello".to_string(), "world".to_string()],
            &HashMap::new(),
            EffectOptions::SilentTest.into(),
        )
        .is_none());
        assert!(print(
            &["hello".to_string(), "world".to_string()],
            &map!["end" => ""],
            EffectOptions::SilentTest.into(),
        )
        .is_none());
        assert!(print(
            &["hello".to_string(), "world".to_string()],
            &map!["eol" => ""],
            EffectOptions::SilentTest.into(),
        )
        .is_some());
    }

    #[test]
    fn test_notify() {
        assert!(notify(
            &[],
            &map![
                "body" => "test_notify",
                "appname" => "scrapeycat",
                "title" => "info",
                "icon" => "lightbulb.svg",
                "sound" => "ding.wav"
            ],
            EffectOptions::SilentTest.into(),
        )
        .is_none());
    }
}
