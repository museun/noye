use crate::test::*;
use httptest::{matchers::*, responders::*, Expectation, Server};

#[tokio::test]
async fn video() {
    set_snapshot_path();

    let server = Server::run();

    let videos = &["23960970", "220883711", "16750764"];
    for video in videos {
        server.expect(
            Expectation::matching(request::method_path(
                "GET",
                format!("/video/{}/config", video),
            ))
            .times(..)
            .respond_with(
                status_code(200).body(
                    std::fs::read_to_string(format!("./snapshots/inputs/vimeo/{}.json", video))
                        .unwrap(),
                ),
            ),
        );
    }

    for video in videos {
        let resp = TestEnv::new(format!("https://vimeo.com/{}", video))
            .insert(super::client::VimeoClient::with_ep(server.url_str("")))
            .execute(super::hear_video)
            .await;

        insta::assert_yaml_snapshot!(resp.get_say::<responses::Vimeo>(), {
            ".owner" => "[owner]",
            ".title" => "[title]",
        });
        resp.expect_empty();
    }
}
