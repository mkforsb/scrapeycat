use std::collections::HashMap;

use notify_rust::Notification;

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

pub enum Mode {
    Normal,
    SilentTest,
}

pub enum EffectOptions {
    Default,
    WithMode(Mode),
}

impl EffectOptions {
    pub fn is_silent_test(&self) -> bool {
        matches!(self, EffectOptions::WithMode(Mode::SilentTest))
    }
}

pub type EffectArgs<'a> = &'a [String];
pub type EffectKwArgs<'a> = &'a HashMap<String, String>;
pub type EffectSignature = fn(EffectArgs, EffectKwArgs, EffectOptions) -> Option<Error>;

pub fn print(args: EffectArgs, kwargs: EffectKwArgs, opts: EffectOptions) -> Option<Error> {
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

pub fn notify(args: EffectArgs, kwargs: EffectKwArgs, opts: EffectOptions) -> Option<Error> {
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
            EffectOptions::WithMode(Mode::SilentTest),
        )
        .is_none());
        assert!(print(
            &["hello".to_string(), "world".to_string()],
            &map!["end" => ""],
            EffectOptions::WithMode(Mode::SilentTest),
        )
        .is_none());
        assert!(print(
            &["hello".to_string(), "world".to_string()],
            &map!["eol" => ""],
            EffectOptions::WithMode(Mode::SilentTest),
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
            EffectOptions::WithMode(Mode::SilentTest),
        )
        .is_none());
    }
}
