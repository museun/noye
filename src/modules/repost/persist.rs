#[derive(Debug, Clone)]
pub struct LinkItem {
    pub link: String,
    pub nick: String,
    pub room: String,
    pub time: time::OffsetDateTime,
    pub posts: i64,
    pub ignored: bool,
}

pub struct Channel<'a> {
    name: &'a str,
}

impl<'a> Channel<'a> {
    pub const fn new(name: &'a str) -> Self {
        Self { name }
    }

    pub fn get_links(&self) -> Vec<LinkItem> {
        let conn = crate::db::get::<super::RepostTable>();
        let mut stmt = match conn.prepare("SELECT * FROM links WHERE :room = room") {
            Ok(a) => a,
            _ => return Vec::new(),
        };

        let iter = match stmt.query_map_named(
            rusqlite::named_params! {
                ":room": &self.name
            },
            |row| {
                Ok(LinkItem {
                    link: row.get("link")?,
                    nick: row.get("nick")?,
                    room: row.get("room")?,
                    time: serde_json::from_slice(&row.get::<_, Vec<u8>>("time")?).unwrap(),
                    posts: row.get("posts")?,
                    ignored: row.get("ignored")?,
                })
            },
        ) {
            Ok(result) => result,
            _ => return Vec::new(),
        };

        iter.flatten().collect()
    }

    pub fn ignore_link(&self, link: &str) -> anyhow::Result<bool> {
        let link = link.trim();

        let conn = crate::db::get::<super::RepostTable>();
        conn.execute_named(
            "UPDATE links SET ignored = :ignored WHERE link = :link AND room = :room",
            rusqlite::named_params! {
                ":ignored": 1,
                ":link": link,
                ":room": &self.name,
            },
        )?;

        log::trace!("ignoring: {} | {}", link, &self.name);
        match conn.execute_named(
            "INSERT INTO `ignored_links` (link, room) VALUES (:link, :room)",
            rusqlite::named_params! {
                ":link": link,
                ":room": &self.name,
            },
        ) {
            Ok(d) if d == 1 => Ok(true),
            Ok(..) | Err(..) => Ok(false),
        }
    }

    pub fn insert_link(&self, nick: &str, link: &str) -> Option<LinkItem> {
        let link = link.trim();

        let mut links = self.get_links();
        let pos = links.iter().position(|d| d.link == link);

        match pos {
            Some(pos) => {
                let old = links.swap_remove(pos);
                self.update_old(nick, link, &old);
                Some(old)
            }
            None => {
                self.insert_new(nick, link);
                None
            }
        }
    }

    pub fn is_ignored(&self, link: &str) -> bool {
        let link = link.trim();

        let conn = crate::db::get::<super::RepostTable>();
        let mut stmt = conn
            .prepare("SELECT * FROM ignored_links WHERE link = :link AND room = :room")
            .unwrap();
        stmt.exists(rusqlite::params! {
           link,
           &self.name,
        })
        .unwrap()
    }

    fn insert_new(&self, nick: &str, link: &str) {
        let link = link.trim();
        let ignored = self.is_ignored(link);

        let conn = crate::db::get::<super::RepostTable>();
        let n = conn
            .execute_named(
                r#"
                INSERT INTO links ( 
                    link, nick, room, time, posts, ignored 
                ) VALUES ( 
                    :link, :nick, :room, :time, :posts, :ignored 
                )
                "#,
                rusqlite::named_params! {
                    ":link": link,
                    ":nick": nick,
                    ":room": &self.name,
                    ":time": serde_json::to_vec(&time::OffsetDateTime::now_utc()).unwrap(),
                    ":posts": 1,
                    ":ignored": ignored,
                },
            )
            .unwrap();
        debug_assert_eq!(n, 1, "1 row should have been updated");
    }

    fn update_old(&self, nick: &str, link: &str, old: &LinkItem) {
        let link = link.trim();

        let ignored = self.is_ignored(link);
        let conn = crate::db::get::<super::RepostTable>();
        let n = conn
            .execute_named(
                r#"
                UPDATE links SET
                    link = :link,
                    nick = :nick,
                    room = :room,
                    time = :time,
                    posts = :posts,
                    ignored = :ignored
                WHERE
                    link = :old
                AND
                    room = :room
                "#,
                rusqlite::named_params! {
                    ":link": link,
                    ":nick": nick,
                    ":room": &self.name,
                    ":time": serde_json::to_vec(&time::OffsetDateTime::now_utc()).unwrap(),
                    ":posts": old.posts + 1,
                    ":ignored": ignored,
                    ":old": &old.link
                },
            )
            .unwrap();
        debug_assert_eq!(n, 1, "1 row should have been updated");
    }
}
