#[allow(dead_code, clippy::missing_const_for_fn)]
pub fn type_name_of_val<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

pub fn type_name<T>() -> &'static str {
    fn reduce_type_name(mut input: &str) -> &str {
        // this is .. totally not something you should do
        fn trim_type(input: &str) -> &str {
            let mut n = input.len();
            let left = input
                .chars()
                .take_while(|&c| {
                    if c == '<' {
                        n -= 1;
                    }
                    !c.is_ascii_uppercase()
                })
                .count();
            &input[left..n]
        }

        let original = input;
        loop {
            let start = input.len();
            input = trim_type(input);
            if input.contains('<') {
                input = trim_type(&input[1..]);
            }
            match input.len() {
                0 => break original,
                d if d == start => break input,
                _ => {}
            }
        }
    }

    reduce_type_name(std::any::type_name::<T>())
}

use std::convert::TryFrom;
use std::time::Duration;

pub trait CommaSeparated {
    fn with_commas(&self) -> String;
}

pub trait FileSize {
    fn as_file_size(&self) -> String;
}

macro_rules! file_size_for {
    ($($ty:ty),*) => {
        $( #[allow(clippy::use_self)] impl FileSize for $ty {
            fn as_file_size(&self) -> String {
                const SIZES: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
                let mut order = 0;
                let mut size = (*self) as f64;
                while size >= 1024.0 && order + 1 < SIZES.len() {
                    order += 1;
                    size /= 1024.0
                }
                format!("{:.2} {}", size, SIZES[order])
            }
        })*
    };
}

file_size_for!(u64, i64);

pub trait Timestamp {
    fn as_timestamp(&self) -> String;
    fn as_readable_time(&self) -> String;
}

impl Timestamp for time::Duration {
    fn as_timestamp(&self) -> String {
        std::time::Duration::try_from(*self).unwrap().as_timestamp()
    }
    fn as_readable_time(&self) -> String {
        readable_time(self.whole_seconds() as _)
    }
}

impl Timestamp for Duration {
    fn as_timestamp(&self) -> String {
        let time = self.as_secs();
        let hours = time / (60 * 60);
        let minutes = (time / 60) % 60;
        let seconds = time % 60;
        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    fn as_readable_time(&self) -> String {
        readable_time(self.as_secs())
    }
}

#[inline]
fn readable_time(mut secs: u64) -> String {
    const TABLE: [(&str, u64); 4] = [
        ("days", 86400),
        ("hours", 3600),
        ("minutes", 60),
        ("seconds", 1),
    ];

    fn pluralize(s: &&str, n: u64) -> String {
        format!("{} {}", n, if n > 1 { s } else { &s[..s.len() - 1] })
    }

    let mut time = vec![];
    for (name, d) in &TABLE {
        let div = secs / d;
        if div > 0 {
            time.push(pluralize(name, div));
            secs -= d * div;
        }
    }

    let len = time.len();
    if len > 1 {
        if len > 2 {
            for e in &mut time.iter_mut().take(len - 2) {
                e.push_str(",")
            }
        }
        time.insert(len - 1, "and".into());
    }
    time.join(" ")
}

pub trait Iso8601 {
    fn from_iso8601(&self) -> i64;
}

impl Iso8601 for String {
    fn from_iso8601(&self) -> i64 {
        self.as_str().from_iso8601()
    }
}

impl Iso8601 for &str {
    fn from_iso8601(&self) -> i64 {
        let parse = |s, e| -> i64 { self[s + 1..e].parse().unwrap_or_default() };
        let (period, _) = self
            .chars()
            .enumerate()
            .fold((0, 0), |(a, p), (i, c)| match c {
                c if c.is_numeric() => (a, p),
                'H' => (a + parse(p, i) * 60 * 60, i),
                'M' => (a + parse(p, i) * 60, i),
                'S' => (a + parse(p, i), i),
                _ => (a, i), // P | T
            });
        period
    }
}

macro_rules! impl_commas_for {
    ($($ty:ty)*) => {
        $(
            impl CommaSeparated for $ty {
                fn with_commas(&self) -> String {
                    fn commas(n: $ty, w: &mut impl std::fmt::Write) {
                        if n < 1000 {
                            write!(w, "{}", n).unwrap();
                            return;
                        }
                        commas(n / 1000, w);
                        write!(w, ",{:03}", n % 1000).unwrap();
                    }
                    let mut buf = String::new();
                    commas(*self, &mut buf);
                    buf
                }
            }
        )*
    };
}

macro_rules! impl_timestamp_for {
    ($($ty:ty)*) => {
        $(
            impl Timestamp for $ty {
                fn as_timestamp(&self) -> String {
                    Duration::from_secs(*self as _).as_timestamp()
                }
                fn as_readable_time(&self) -> String {
                    Duration::from_secs(*self as _).as_readable_time()
                }
            }
        )*
    }
}

impl_commas_for!(
    u16 u32 u64 u128 usize //
    i16 i32 i64 i128 isize //
);

impl_timestamp_for!(
    u64 usize //
    i64 isize //
);
