use super::*;
use crate::test::*;

#[test]
fn channel() {
    let _db = crate::db::get::<RepostTable>();

    let links = &["http://example.com", "http://example.com/foobar"];
    let nicks = &["foo", "bar"];
    let tests = &["#testing", "#foobar"];

    for test in tests {
        let channel = persist::Channel::new(test);
        for link in links {
            assert_eq!(channel.is_ignored(link), false);
            assert_eq!(channel.ignore_link(link).unwrap(), true);
            assert_eq!(channel.is_ignored(link), true);
        }
        for link in links {
            assert_eq!(channel.ignore_link(link).unwrap(), false);
            assert_eq!(channel.is_ignored(link), true);
        }
        assert!(channel.get_links().is_empty());

        // doesn't exist
        for (link, nick) in links.iter().zip(nicks.iter()) {
            assert!(channel.insert_link(nick, link).is_none());
        }

        // already exists
        for (link, nick) in links.iter().zip(nicks.iter()) {
            let _ = channel.insert_link(nick, link).unwrap();
        }

        assert_eq!(channel.get_links().len(), 2);
    }
}

#[tokio::test]
async fn ignore_link() {
    set_snapshot_path();

    let _db = crate::db::get_connection();

    let responses = TestEnv::new("!ignore http://example.com")
        .execute(super::ignore_link)
        .await;
    insta::assert_yaml_snapshot!(responses.get_reply::<responses::Builtin>());
    responses.expect_empty();

    let responses = TestEnv::new("!ignore http://example.com")
        .owner()
        .execute(super::ignore_link)
        .await;
    insta::assert_yaml_snapshot!(responses.get_reply::<responses::Repost>());
    responses.expect_empty();

    let responses = TestEnv::new("!ignore http://example.com")
        .owner()
        .execute(super::ignore_link)
        .await;
    insta::assert_yaml_snapshot!(responses.get_reply::<responses::Repost>());
    responses.expect_empty();

    let responses = TestEnv::new("!ignore")
        .owner()
        .execute(super::ignore_link)
        .await;
    insta::assert_yaml_snapshot!(responses.get_reply::<responses::Repost>());
    responses.expect_empty();
}

#[tokio::test]
async fn repost_shame() {
    set_snapshot_path();
    let _db = crate::db::get_connection();

    let responses = TestEnv::new("http://example.com")
        .execute(super::repost_shame)
        .await;
    responses.expect_empty();

    let responses = TestEnv::new("http://example.com")
        .config(|config| config.modules.repost.staleness = "7d".into())
        .execute(super::repost_shame)
        .await;
    insta::assert_yaml_snapshot!(responses.get_say::<responses::Repost>());
    responses.expect_empty();

    let responses = TestEnv::new("http://example.com")
        .user("foobar")
        .config(|config| config.modules.repost.staleness = "7d".into())
        .execute(super::repost_shame)
        .await;
    insta::assert_yaml_snapshot!(responses.get_say::<responses::Repost>());
    responses.expect_empty();
}
